use futures_util::StreamExt;
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, error::SendError},
    task::JoinHandle,
};
use yawc::{Frame, MaybeTlsStream, WebSocket};

use crate::client::session::{config::Config, receiver::receiver, sender::sender};

async fn handle_connection(
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    request_receiver: mpsc::Receiver<Frame>,
    msg_sender: mpsc::Sender<bytes::Bytes>,
    config: Config,
) {
    let (sink, stream) = ws.split();
    let receiver = receiver(stream, msg_sender, config.silence_timeout);
    let sender = sender(sink, request_receiver, config);
    // Wait for the first to complete as that suggests something failed
    tokio::select! {
        () = sender => (),
        () = receiver => (),
    };
}

pub(crate) struct ReadySession {
    handle: JoinHandle<()>,
    sender: mpsc::Sender<Frame>,
    receiver: mpsc::Receiver<bytes::Bytes>,
}

impl ReadySession {
    pub(super) fn new(ws: WebSocket<MaybeTlsStream<TcpStream>>, config: Config) -> Self {
        let (sender, request_receiver) = mpsc::channel::<Frame>(32);
        let (msg_sender, receiver) = mpsc::channel::<bytes::Bytes>(1024);
        let handle = tokio::spawn(async move {
            handle_connection(ws, request_receiver, msg_sender, config).await;
        });
        ReadySession {
            handle,
            sender,
            receiver,
        }
    }

    #[allow(unused)]
    pub(crate) async fn send(&self, frame: Frame) -> Result<(), SendError<Frame>> {
        self.sender.send(frame).await
    }

    pub(crate) fn sender(&self) -> &mpsc::Sender<Frame> {
        &self.sender
    }

    pub(crate) async fn recv(&mut self) -> Option<bytes::Bytes> {
        self.receiver.recv().await
    }
}

impl Drop for ReadySession {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
