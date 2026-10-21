use std::time::Duration;

use futures_util::{SinkExt, stream::SplitSink};
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time::{Interval, MissedTickBehavior, interval},
};
use yawc::{Frame, MaybeTlsStream, WebSocket};

use crate::client::session::{config::Config, message::Messages};

fn ping_ticker() -> Interval {
    let mut interval = interval(Duration::from_mins(1)); // Ping every 60s
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    interval.reset(); // Remove the immediate tick

    interval
}

enum Action {
    Send(Option<Frame>),
    SendPing,
}

/// Repeatedly forwards [`Frame`] from a [`Receiver`](mpsc::Receiver) to a WebSocket stream.
///
/// Additionally, ping messages are sent into the WebSocket stream at an interval.
///
/// # Cancellation safety
/// Cancellation prevents any more messages from being sent. Any queued messages are dropped and
/// will be lost.
pub(super) async fn sender(
    mut sink: SplitSink<WebSocket<MaybeTlsStream<TcpStream>>, Frame>,
    mut receiver: mpsc::Receiver<Frame>,
    config: Config,
) {
    let mut ping_ticker = ping_ticker();
    loop {
        let action = tokio::select! {
            biased;
            frame = receiver.recv() => Action::Send(frame),
            _ = ping_ticker.tick() => Action::SendPing,
        };

        let result = match action {
            Action::Send(Some(frame)) => sink.send(frame).await.map_err(|_| ()),
            Action::SendPing => sink.send(Frame::text(config.ping())).await.map_err(|_| ()),
            Action::Send(None) => Err(()),
        };
        if result.is_err() {
            return;
        }
    }
}
