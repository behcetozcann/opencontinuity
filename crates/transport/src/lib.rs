//! Eşler arası şifreli kanal: karşılıklı TLS 1.3 + `Hello` el sıkışması.

mod verify;

use std::sync::Arc;

use anyhow::{Context, bail};
use oc_pairing::{CERT_NAME, Fingerprint, Identity, device_id_of, fingerprint_of};
use oc_protocol::{Message, VERSION, read_message, write_message};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, CommonState, ServerConfig};
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};

pub use verify::Trust;

pub const DEFAULT_PORT: u16 = 47800;
const EXPORTER_LABEL: &[u8] = b"oc-sas";

pub type Stream = TlsStream<TcpStream>;

pub struct Session {
    stream: Stream,
    pub peer_name: String,
    pub peer_fingerprint: Fingerprint,
    /// Eşin dinlediği port; ona geri bağlanmak için.
    pub peer_listen_port: u16,
    /// Bu TLS oturumuna özgü anahtar türevi; SAS kodunu oturuma bağlar.
    pub exporter: [u8; 32],
}

impl Session {
    pub async fn send(&mut self, message: &Message) -> anyhow::Result<()> {
        write_message(&mut self.stream, message).await
    }

    pub async fn recv(&mut self) -> anyhow::Result<Message> {
        read_message(&mut self.stream).await
    }

    pub fn split(self) -> (ReadHalf<Stream>, WriteHalf<Stream>) {
        tokio::io::split(self.stream)
    }
}

/// `listen_port` bizim dinlediğimiz port; `Hello` ile eşe bildirilir.
pub async fn connect(
    address: impl ToSocketAddrs,
    identity: &Identity,
    trust: Trust,
    listen_port: u16,
) -> anyhow::Result<Session> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verify::PeerVerifier::new(trust, provider)))
        .with_client_auth_cert(vec![certificate(identity)], private_key(identity))?;
    let tcp = TcpStream::connect(address).await.context("bağlantı kurulamadı")?;
    tcp.set_nodelay(true)?;
    let server_name = ServerName::try_from(CERT_NAME)?;
    let tls = TlsConnector::from(Arc::new(config)).connect(server_name, tcp).await?;
    let (fingerprint, exporter) = session_secrets(tls.get_ref().1, |out| {
        tls.get_ref().1.export_keying_material(out, EXPORTER_LABEL, None).map(|_| ())
    })?;
    hello(TlsStream::from(tls), identity, fingerprint, exporter, Local { trust, listen_port }).await
}

pub async fn accept(tcp: TcpStream, identity: &Identity, trust: Trust, listen_port: u16) -> anyhow::Result<Session> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(Arc::new(verify::PeerVerifier::new(trust, provider)))
        .with_single_cert(vec![certificate(identity)], private_key(identity))?;
    tcp.set_nodelay(true)?;
    let tls = TlsAcceptor::from(Arc::new(config)).accept(tcp).await?;
    let (fingerprint, exporter) = session_secrets(tls.get_ref().1, |out| {
        tls.get_ref().1.export_keying_material(out, EXPORTER_LABEL, None).map(|_| ())
    })?;
    hello(TlsStream::from(tls), identity, fingerprint, exporter, Local { trust, listen_port }).await
}

fn session_secrets(
    state: &CommonState,
    export: impl FnOnce(&mut [u8; 32]) -> Result<(), rustls::Error>,
) -> anyhow::Result<(Fingerprint, [u8; 32])> {
    let certificate = state.peer_certificates().and_then(|chain| chain.first()).context("eş sertifika göndermedi")?;
    let mut exporter = [0; 32];
    export(&mut exporter)?;
    Ok((fingerprint_of(certificate), exporter))
}

struct Local {
    trust: Trust,
    listen_port: u16,
}

async fn hello(
    mut stream: Stream,
    identity: &Identity,
    peer_fingerprint: Fingerprint,
    exporter: [u8; 32],
    local: Local,
) -> anyhow::Result<Session> {
    let own_pairing = local.trust == Trust::Pairing;
    let own = Message::Hello {
        version: VERSION,
        device_id: identity.device_id(),
        name: identity.name.clone(),
        listen_port: local.listen_port,
        pairing: own_pairing,
    };
    write_message(&mut stream, &own).await?;
    let Message::Hello { version, device_id, name, listen_port, pairing } = read_message(&mut stream).await? else {
        bail!("eş Hello ile başlamadı");
    };
    if version != VERSION {
        bail!("protokol sürümü uyuşmuyor: eş {version}, biz {VERSION}");
    }
    if device_id != device_id_of(&peer_fingerprint) {
        bail!("eşin cihaz kimliği sertifikasıyla uyuşmuyor");
    }
    // Eşleşmesi kaldırılmış cihazın eski eşi, eşleştirme bekleyen tarafa bağlanıp
    // akışı bozmasın (ve tersi).
    if pairing != own_pairing {
        bail!("eş {} modunda", if pairing { "eşleştirme" } else { "senkron" });
    }
    Ok(Session { stream, peer_name: name, peer_fingerprint, peer_listen_port: listen_port, exporter })
}

fn certificate(identity: &Identity) -> CertificateDer<'static> {
    CertificateDer::from(identity.cert_der.clone())
}

fn private_key(identity: &Identity) -> PrivateKeyDer<'static> {
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key_der.clone()))
}
