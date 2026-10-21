use backoff::{ExponentialBackoff, backoff::Backoff};
use tokio::sync::mpsc;
use tracing::info;
use yawc::Frame;

use crate::client::{
    Client,
    config::Config,
    receiver::Receiver,
    sender::Sender,
    session::{Config as SessionConfig, IdleSession},
};

enum SessionError {
    ConnectionError,
    InitError,
    SessionClosed,
    ChannelClosed,
}

async fn create_session(
    config: SessionConfig,
    request_receiver: &mut mpsc::Receiver<Frame>,
    message_sender: &mpsc::Sender<bytes::Bytes>,
) -> Result<(), SessionError> {
    let idle_session = IdleSession::new(config);
    let connected_session = idle_session
        .connect()
        .await
        .map_err(|_| SessionError::ConnectionError)?;
    let mut session = connected_session
        .init()
        .await
        .map_err(|_| SessionError::InitError)?;

    // Purge all requests that were made before the session was ready
    while request_receiver.try_recv().is_ok() {}

    let sender = session.sender().clone();
    let forward_request = async move {
        loop {
            let Some(message) = request_receiver.recv().await else {
                return Err::<(), _>(SessionError::ChannelClosed);
            };
            let Ok(()) = sender.send(message).await else {
                return Err::<(), _>(SessionError::SessionClosed);
            };
        }
    };

    let forward_response = async move {
        loop {
            let Some(bytes) = session.recv().await else {
                return Err::<(), _>(SessionError::SessionClosed);
            };
            let Ok(()) = message_sender.send(bytes).await else {
                return Err::<(), _>(SessionError::ChannelClosed);
            };
        }
    };

    tokio::select! {
        result = forward_request => result,
        result = forward_response => result,
    }
}

pub(crate) struct IdleClient {
    config: Config,
}

impl IdleClient {
    pub(crate) fn new(config: Config) -> Self {
        IdleClient { config }
    }

    pub(crate) fn connect(self) -> Client {
        let (sender, mut request_receiver) = mpsc::channel::<Frame>(8);
        let (message_sender, receiver) = mpsc::channel::<bytes::Bytes>(1024);
        let session_config = self.config.session.clone();

        let handle = tokio::spawn(async move {
            let Config {
                session: session_config,
                cooldown: cooldown_config,
            } = self.config;
            let mut backoff: ExponentialBackoff = cooldown_config.into();

            loop {
                info!("Creating session");
                let result = create_session(
                    session_config.clone(),
                    &mut request_receiver,
                    &message_sender,
                )
                .await;
                match result {
                    Err(SessionError::ChannelClosed) => {
                        return; // Channel closed, fatal
                    }
                    Err(SessionError::InitError | SessionError::ConnectionError) => {}
                    // Reset cooldown as session was successful
                    Ok(()) | Err(SessionError::SessionClosed) => backoff.reset(),
                }
                let Some(duration) = backoff.next_backoff() else {
                    return; // No more retries allowed, fatal
                };
                info!("Session ended, retrying in {:.2}s", duration.as_secs_f64());
                tokio::time::sleep(duration).await;
            }
        });

        Client {
            handle,
            sender: Sender::new(sender, session_config),
            receiver: Receiver::new(receiver),
        }
    }
}
