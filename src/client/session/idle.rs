use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error, client::IntoClientRequest, http::HeaderValue},
};

use crate::client::session::{
    config::{Config, ORIGIN},
    connected::ConnectedSession,
};

pub(crate) struct IdleSession {
    config: Config,
}

impl IdleSession {
    pub(crate) fn new(config: Config) -> Self {
        IdleSession { config }
    }

    /// Initiates a WebSocket connection to the URL specified by [`Config`].
    ///
    /// # Cancellation safety
    /// Cancellation interrupts the connection request.
    pub(crate) async fn connect(self) -> Result<ConnectedSession, Error> {
        let url = self.config.url.as_str();
        let mut request = url.into_client_request()?;

        let headers = request.headers_mut();
        headers.insert("Origin", HeaderValue::from_str(ORIGIN)?);

        let (ws, _) = connect_async(request).await?;
        Ok(ConnectedSession::new(ws, self.config))
    }
}
