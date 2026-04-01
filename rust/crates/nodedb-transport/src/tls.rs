use std::sync::Arc;

use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::TlsAcceptor;

use crate::error::TransportError;

/// Ensure the rustls ring CryptoProvider is installed (idempotent).
fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// A TLS certificate + private key pair.
pub struct TlsCertKey {
    pub cert_pem: String,
    pub key_pem: String,
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

/// Generate a self-signed TLS certificate for localhost / peer communication.
pub fn generate_self_signed_cert() -> Result<TlsCertKey, TransportError> {
    let mut params = CertificateParams::new(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
    ]).map_err(|e| TransportError::Tls(e.to_string()))?;

    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String("NodeDB Peer".to_string()),
    );

    let key_pair = KeyPair::generate()
        .map_err(|e| TransportError::Tls(e.to_string()))?;

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| TransportError::Tls(e.to_string()))?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();

    Ok(TlsCertKey {
        cert_pem,
        key_pem,
        cert_der,
        key_der,
    })
}

/// Build a TLS server config from cert + key.
pub fn build_server_tls_config(cert_key: &TlsCertKey) -> Result<Arc<rustls::ServerConfig>, TransportError> {
    ensure_crypto_provider();
    let cert = CertificateDer::from(cert_key.cert_der.clone());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert_key.key_der.clone()));

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| TransportError::Tls(e.to_string()))?;

    Ok(Arc::new(config))
}

/// Build a TLS client config that accepts self-signed certificates.
/// For peer-to-peer communication where both sides use self-signed certs.
pub fn build_client_tls_config() -> Result<Arc<rustls::ClientConfig>, TransportError> {
    ensure_crypto_provider();
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCert))
        .with_no_client_auth();

    Ok(Arc::new(config))
}

/// Build a TLS acceptor from server config.
pub fn build_tls_acceptor(server_config: Arc<rustls::ServerConfig>) -> TlsAcceptor {
    TlsAcceptor::from(server_config)
}

/// Certificate verifier that accepts any certificate.
/// Used for self-signed peer-to-peer TLS where identity is verified at the
/// application layer (Hello/HelloAck handshake with Ed25519 keys).
#[derive(Debug)]
struct AcceptAnyCert;

impl rustls::client::danger::ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_self_signed_cert_succeeds() {
        let cert_key = generate_self_signed_cert().unwrap();
        assert!(!cert_key.cert_pem.is_empty());
        assert!(!cert_key.key_pem.is_empty());
        assert!(!cert_key.cert_der.is_empty());
        assert!(!cert_key.key_der.is_empty());
    }

    #[test]
    fn build_server_config_succeeds() {
        let cert_key = generate_self_signed_cert().unwrap();
        let config = build_server_tls_config(&cert_key).unwrap();
        assert!(Arc::strong_count(&config) == 1);
    }

    #[test]
    fn build_client_config_succeeds() {
        let config = build_client_tls_config().unwrap();
        assert!(Arc::strong_count(&config) == 1);
    }
}
