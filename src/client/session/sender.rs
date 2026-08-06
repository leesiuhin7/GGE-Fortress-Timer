use std::time::Duration;

use futures_util::{SinkExt, stream::SplitSink};
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time::{Interval, MissedTickBehavior, interval},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::client::session::{config::Config, message::Messages};

fn ping_ticker() -> Interval {
    let mut interval = interval(Duration::from_mins(1)); // Ping every 60s
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    interval.reset(); // Remove the immediate tick

    interval
}

enum Action {
    Send(Option<Message>),
    SendPing,
}

/// Repeatedly forwards [`Message`] from a [`Receiver`](mpsc::Receiver) to a WebSocket stream.
///
/// Additionally, ping messages are sent into the WebSocket stream at an interval.
///
/// # Cancellation safety
/// Cancellation prevents any more messages from being sent. Any queued messages are dropped and
/// will be lost.
pub(super) async fn sender(
    mut sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut receiver: mpsc::Receiver<Message>,
    config: Config,
) {
    let mut ping_ticker = ping_ticker();
    loop {
        let action = tokio::select! {
            biased;
            message = receiver.recv() => Action::Send(message),
            _ = ping_ticker.tick() => Action::SendPing,
        };

        let result = match action {
            Action::Send(Some(message)) => sink.send(message).await.map_err(|_| ()),
            Action::SendPing => sink.send(config.ping().into()).await.map_err(|_| ()),
            Action::Send(_) => Err(()),
        };
        if result.is_err() {
            return;
        }
    }
}
