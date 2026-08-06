mod config;
mod cooldown;
mod idle;
mod receiver;
mod sender;
mod session;

use tokio::task::JoinHandle;

pub(crate) use crate::client::{
    config::Config,
    cooldown::Config as CooldownConfig,
    idle::IdleClient,
    receiver::{Error, Receiver, Response},
    sender::{Command, Sender},
    session::Config as SessionConfig,
};

pub(crate) struct Client {
    pub(self) handle: JoinHandle<()>,
    pub(self) sender: Sender,
    pub(self) receiver: Receiver,
}

impl Client {
    pub(crate) async fn send(&self, command: Command) -> Result<(), ()> {
        self.sender.send(command).await
    }

    pub(crate) fn sender(&self) -> Sender {
        self.sender.clone()
    }

    pub(crate) async fn recv(&mut self) -> Result<Vec<Response>, ()> {
        loop {
            match self.receiver.recv().await {
                Ok(responses) => {
                    return Ok(responses);
                }
                Err(Error::Recv) => {
                    return Err(());
                }
                Err(Error::Utf8) => {} // Non-fatal hence wait for next one
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.handle.abort(); // Abort task to ensure underlying connection is closed
    }
}
