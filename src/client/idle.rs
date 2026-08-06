use backoff::{ExponentialBackoff, backoff::Backoff};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Bytes, Message};

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
    request_receiver: &mut mpsc::Receiver<Message>,
    message_sender: &mpsc::Sender<Bytes>,
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
        let (sender, mut request_receiver) = mpsc::channel::<Message>(32);
        let (message_sender, receiver) = mpsc::channel::<Bytes>(1024);
        let session_config = self.config.session.clone();

        let handle = tokio::spawn(async move {
            let Config {
                session: session_config,
                cooldown: cooldown_config,
            } = self.config;
            let mut backoff: ExponentialBackoff = cooldown_config.into();

            loop {
                println!("create_session");
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
