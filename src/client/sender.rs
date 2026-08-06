use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::client::session::Config;

pub(crate) struct Command {
    pub(crate) command: String,
    pub(crate) payload: String,
}

#[derive(Clone)]
pub(crate) struct Sender {
    sender: mpsc::Sender<Message>,
    config: Config,
}

impl Sender {
    pub(super) fn new(sender: mpsc::Sender<Message>, config: Config) -> Self {
        Sender { sender, config }
    }

    pub(crate) async fn send(&self, command: Command) -> Result<(), ()> {
        let server = self.config.server.as_str();
        let Command { command, payload } = command;
        let message = format!("%xt%{server}%{command}%1%{payload}%");

        // Simply drop the command if send fails as it is fatal
        self.sender
            .send(Message::Text(message.into()))
            .await
            .map_err(|_| ())
    }
}
