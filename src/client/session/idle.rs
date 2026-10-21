use crate::client::session::{
    config::{Config, ORIGIN},
    connect::{Builder, ConnectError, connect_with_no_verification},
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
    pub(crate) async fn connect(self) -> Result<ConnectedSession, ConnectError> {
        let request = Builder::new(&self.config.url).header("Origin", ORIGIN);

        let ws = connect_with_no_verification(request).await?;
        Ok(ConnectedSession::new(ws, self.config))
    }
}
