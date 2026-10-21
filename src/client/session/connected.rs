use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use yawc::{Frame, MaybeTlsStream, WebSocket, WebSocketError};

use crate::client::session::{
    config::Config,
    message::{AUTO_JOIN, Messages, ROUND_TRIP, VERSION_CHECK},
    ready::ReadySession,
};

pub(crate) struct ConnectedSession {
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    config: Config,
}

impl ConnectedSession {
    pub(super) fn new(ws: WebSocket<MaybeTlsStream<TcpStream>>, config: Config) -> Self {
        ConnectedSession { ws, config }
    }

    pub(crate) async fn init(mut self) -> Result<ReadySession, WebSocketError> {
        self.send(VERSION_CHECK).await?;
        self.wait_recv(1).await?;

        self.send(self.config.login()).await?;
        self.wait_recv(3).await?;

        self.send(AUTO_JOIN).await?;
        self.wait_recv(1).await?;

        self.send(ROUND_TRIP).await?;
        self.send(self.config.init_round_house_kick()).await?;
        self.wait_recv(2).await?;

        self.send(self.config.search_username()).await?;
        self.wait_recv(1).await?;

        self.send(self.config.user_login()).await?;

        Ok(ReadySession::new(self.ws, self.config))
    }

    async fn wait_recv(&mut self, count: u8) -> Result<(), WebSocketError> {
        for _ in 0..count {
            if self.ws.next().await.is_none() {
                return Err(WebSocketError::ConnectionClosed);
            }
        }
        Ok(())
    }

    async fn send(&mut self, message: impl Into<bytes::Bytes>) -> Result<(), WebSocketError> {
        self.ws.send(Frame::text(message)).await
    }
}
