use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Error, Message, Utf8Bytes},
};

use crate::client::session::{
    config::Config,
    message::{AUTO_JOIN, Messages, ROUND_TRIP, VERSION_CHECK},
    ready::ReadySession,
};

pub(crate) struct ConnectedSession {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    config: Config,
}

impl ConnectedSession {
    pub(super) fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>, config: Config) -> Self {
        ConnectedSession { ws, config }
    }

    pub(crate) async fn init(mut self) -> Result<ReadySession, Error> {
        self.send(VERSION_CHECK.into()).await?;
        self.wait_recv(1).await?;

        self.send(self.config.login().into()).await?;
        self.wait_recv(3).await?;

        self.send(AUTO_JOIN.into()).await?;
        self.wait_recv(1).await?;

        self.send(ROUND_TRIP.into()).await?;
        self.send(self.config.init_round_house_kick().into())
            .await?;
        self.wait_recv(2).await?;

        self.send(self.config.search_username().into()).await?;
        self.wait_recv(1).await?;

        self.send(self.config.user_login().into()).await?;

        Ok(ReadySession::new(self.ws, self.config))
    }

    async fn wait_recv(&mut self, count: u8) -> Result<(), Error> {
        for _ in 0..count {
            match self.ws.next().await {
                Some(Ok(_)) => Ok(()),
                Some(Err(error)) => Err(error),
                None => Err(Error::ConnectionClosed),
            }?;
        }
        Ok(())
    }

    async fn send(&mut self, message: Utf8Bytes) -> Result<(), Error> {
        self.ws.send(Message::Text(message)).await
    }
}
