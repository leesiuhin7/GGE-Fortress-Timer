use futures_util::StreamExt;
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, error::SendError},
    task::JoinHandle,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Bytes, Message},
};

use crate::client::session::{config::Config, receiver::receiver, sender::sender};

async fn handle_connection(
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    request_receiver: mpsc::Receiver<Message>,
    msg_sender: mpsc::Sender<Bytes>,
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
    sender: mpsc::Sender<Message>,
    receiver: mpsc::Receiver<Bytes>,
}

impl ReadySession {
    pub(super) fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>, config: Config) -> Self {
        let (sender, request_receiver) = mpsc::channel::<Message>(32);
        let (msg_sender, receiver) = mpsc::channel::<Bytes>(1024);
        let handle = tokio::spawn(async move {
            handle_connection(ws, request_receiver, msg_sender, config).await;
        });
        ReadySession {
            handle,
            sender,
            receiver,
        }
    }

    pub(crate) async fn send(&self, message: Message) -> Result<(), SendError<Message>> {
        self.sender.send(message).await
    }

    pub(crate) fn sender(&self) -> &mpsc::Sender<Message> {
        &self.sender
    }

    pub(crate) async fn recv(&mut self) -> Option<Bytes> {
        self.receiver.recv().await
    }
}

impl Drop for ReadySession {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
