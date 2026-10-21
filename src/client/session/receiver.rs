use std::time::Duration;

use futures_util::{StreamExt, stream::SplitStream};
use tokio::{net::TcpStream, sync::mpsc};
use yawc::{Frame, MaybeTlsStream, OpCode, WebSocket};

use crate::client::session::timeout::Timeout;

enum Action {
    Recv(Option<Frame>),
    Timeout,
}

/// Repeatedly forwards messages from a WebSocket stream to a [`Sender`](mpsc::Sender).
///
/// Forwards WebSocket messages as [`Bytes`](bytes::Bytes) to a [`Sender`](mpsc::Sender) until no
/// messages have been received for a period specified by [`silence_timeout`]
///
/// # Cancellation safety
/// Cancellation guarantees the [`sender`](mpsc::Sender) will not receive any more messages. The
/// remaining messages are therefore lost.
pub(super) async fn receiver(
    mut stream: SplitStream<WebSocket<MaybeTlsStream<TcpStream>>>,
    sender: mpsc::Sender<bytes::Bytes>,
    silence_timeout: Duration,
) {
    let mut timeout = Timeout::new(silence_timeout);
    loop {
        let action = tokio::select! {
            frame = stream.next() => Action::Recv(frame),
            () = timeout.wait() => Action::Timeout,
        };

        match action {
            Action::Recv(Some(frame)) => {
                match frame.opcode() {
                    OpCode::Binary | OpCode::Text => {
                        let bytes = frame.into_payload();
                        timeout.reset(); // Reset timeout as an actual message is received
                        let result = sender.send(bytes).await;
                        if result.is_err() {
                            return;
                        }
                    }
                    OpCode::Close => {
                        return; // Received close frame, exit
                    }
                    _ => {}
                }
            }
            Action::Recv(None) | Action::Timeout => {
                return;
            }
        }
    }
}
