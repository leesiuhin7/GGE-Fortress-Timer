use std::sync::Arc;

use rustls::{
    ClientConfig,
    client::danger::{ServerCertVerified, ServerCertVerifier},
    crypto::{ring::default_provider, verify_tls12_signature, verify_tls13_signature},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
    tungstenite::{Error, client::IntoClientRequest, handshake::client::Response},
};

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

pub(super) async fn connect_with_no_verification<R: IntoClientRequest + Unpin>(
    request: R,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DisableVerification))
        .with_no_client_auth();

    connect_async_tls_with_config(
        request,
        None,
        false,
        Some(Connector::Rustls(Arc::new(config))),
    )
    .await
}
