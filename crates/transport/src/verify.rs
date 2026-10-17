//! Sertifika zinciri yerine parmak izine dayanan doğrulayıcı. El sıkışma
//! imzası her durumda doğrulanır; böylece karşı taraf sertifikanın özel
//! anahtarına sahip olduğunu kanıtlar.

use std::sync::Arc;

use oc_pairing::{Fingerprint, fingerprint_of};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, Error, SignatureScheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Yalnız bu parmak izine sahip eş kabul edilir.
    Pinned(Fingerprint),
    /// Eşleştirme: her sertifika kabul edilir, güven sonradan SAS ile kurulur.
    Pairing,
}

#[derive(Debug)]
pub struct PeerVerifier {
    trust: Trust,
    provider: Arc<CryptoProvider>,
}

impl PeerVerifier {
    pub fn new(trust: Trust, provider: Arc<CryptoProvider>) -> Self {
        Self { trust, provider }
    }

    fn check(&self, certificate: &CertificateDer<'_>) -> Result<(), Error> {
        match self.trust {
            Trust::Pairing => Ok(()),
            Trust::Pinned(expected) if fingerprint_of(certificate) == expected => Ok(()),
            Trust::Pinned(_) => Err(Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure)),
        }
    }

    fn verify_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, certificate, signature, &self.provider.signature_verification_algorithms)
    }

    fn schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

impl ServerCertVerifier for PeerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        self.check(end_entity).map(|()| ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General("TLS 1.2 desteklenmiyor".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.verify_signature(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

impl ClientCertVerifier for PeerVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        self.check(end_entity).map(|()| ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General("TLS 1.2 desteklenmiyor".into()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.verify_signature(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}
