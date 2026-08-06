use std::time::Duration;

use futures_util::{StreamExt, stream::SplitStream};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Bytes, Error, Message},
};

use crate::client::session::timeout::Timeout;

enum Action {
    Recv(Option<Result<Message, Error>>),
    Timeout,
}

/// Repeatedly forwards messages from a WebSocket stream to a [`Sender`](mpsc::Sender).
///
/// Forwards WebSocket messages as [`Bytes`] to a [`Sender`](mpsc::Sender) until no messages have
/// been received for a period specified by [`silence_timeout`]
///
/// # Cancellation safety
/// Cancellation guarantees the [`sender`](mpsc::Sender) will not receive any more messages. The
/// remaining messages are therefore lost.
pub(super) async fn receiver(
    mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    sender: mpsc::Sender<Bytes>,
    silence_timeout: Duration,
) {
    let mut timeout = Timeout::new(silence_timeout);
    loop {
        let action = tokio::select! {
            message = stream.next() => Action::Recv(message),
            () = timeout.wait() => Action::Timeout,
        };

        match action {
            Action::Recv(Some(Ok(message))) => {
                let data = match message {
                    Message::Text(bytes) => Some(bytes.into()),
                    Message::Binary(bytes) => Some(bytes),
                    Message::Close(_) => {
                        return; // Received close frame, exit
                    }
                    _ => None,
                };
                if let Some(bytes) = data {
                    timeout.reset(); // Reset timeout as an actual message is received
                    let result = sender.send(bytes).await;
                    if result.is_err() {
                        return;
                    }
                }
            }
            Action::Recv(_) | Action::Timeout => {
                return;
            }
        }
    }
}
