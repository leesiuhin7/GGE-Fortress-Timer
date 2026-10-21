use std::sync::Arc;

use rustls::{
    ClientConfig,
    client::danger::{ServerCertVerified, ServerCertVerifier},
    crypto::{ring::default_provider, verify_tls12_signature, verify_tls13_signature},
};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use yawc::{HttpRequestBuilder, MaybeTlsStream, Options, WebSocket, WebSocketError};

#[derive(Debug)]
struct DisableVerification;

impl ServerCertVerifier for DisableVerification {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub(super) struct Builder<'a> {
    builder: HttpRequestBuilder,
    url: &'a str,
}

impl<'a> Builder<'a> {
    pub(super) fn new(url: &'a str) -> Self {
        Builder {
            builder: HttpRequestBuilder::new().uri(url),
            url,
        }
    }

    pub(super) fn header(self, key: &str, value: &str) -> Self {
        let builder = self.builder.header(key, value);
        Builder {
            builder,
            url: self.url,
        }
    }

    fn url(&self) -> &str {
        self.url
    }
}

impl From<Builder<'_>> for HttpRequestBuilder {
    fn from(value: Builder<'_>) -> Self {
        value.builder
    }
}

pub(crate) enum ConnectError {
    Url,
    #[allow(unused)]
    WebSocket(WebSocketError),
}

pub(super) async fn connect_with_no_verification(
    builder: Builder<'_>,
) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, ConnectError> {
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DisableVerification))
        .with_no_client_auth();

    let url = builder.url().parse().map_err(|_| ConnectError::Url)?;
    WebSocket::connect(url)
        .with_connector(TlsConnector::from(Arc::new(config)))
        .with_options(Options::default().with_balanced_compression())
        .with_request(builder.into())
        .await
        .map_err(ConnectError::WebSocket)
}
