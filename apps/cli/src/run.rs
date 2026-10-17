//! `oc run`: eşle bağlantıyı ayakta tutar ve panoyu iki yönde senkronlar.

use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use oc_core::SyncState;
use oc_discovery::Discovery;
use oc_pairing::{Peer, PeerStore, device_id_of};
use oc_protocol::{MAX_TEXT_BYTES, Message, read_message, write_message};
use oc_transport::{DEFAULT_PORT, Session, Stream, Trust};
use tokio::io::WriteHalf;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep_until, timeout};

use crate::{clipboard_thread, config};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const DIAL_TIMEOUT: Duration = Duration::from_secs(3);
const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

pub async fn run() -> anyhow::Result<()> {
    let identity = config::identity()?;
    let store = PeerStore::new(&config::dir()?);
    let Some(mut peer) = store.load()? else {
        bail!("eşleşmiş cihaz yok; önce `oc pair` çalıştır");
    };
    let listener = TcpListener::bind(("0.0.0.0", DEFAULT_PORT))
        .await
        .with_context(|| format!("port {DEFAULT_PORT} açılamadı (başka bir `oc run` çalışıyor olabilir)"))?;
    let discovery = Discovery::start(&identity.name, &identity.fingerprint(), DEFAULT_PORT, false)?;
    let mut discovered = discovery.browse()?;
    let mut clipboard = clipboard_thread::spawn()?;

    let mut state = SyncState::new(identity.device_id());
    let peer_device_id = device_id_of(&peer.fingerprint);
    let peer_hex = hex::encode(peer.fingerprint);
    let trust = Trust::Pinned(peer.fingerprint);
    // Yalnız küçük parmak izli taraf bağlanır; çift bağlantı yarışı olmaz.
    let dialer = identity.fingerprint() < peer.fingerprint;
    let mut addresses: Vec<SocketAddr> = peer.last_address.iter().filter_map(|address| address.parse().ok()).collect();
    let mut active: Option<Active> = None;
    let mut backoff = MIN_BACKOFF;
    let mut next_dial = tokio::time::Instant::now();
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);

    log(format!("Çalışıyor. Eş: {} ({} taraf). Çıkmak için Ctrl+C.", peer.name, if dialer { "bağlanan" } else { "bekleyen" }));
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (tcp, remote) = accepted?;
                match timeout(HANDSHAKE_TIMEOUT, oc_transport::accept(tcp, &identity, trust)).await {
                    Ok(Ok(session)) => {
                        remember_address(&store, &mut peer, &mut addresses, remote.ip())?;
                        log(format!("Bağlandı: {} ({})", session.peer_name, remote.ip()));
                        active = activate(session, &state).await;
                    }
                    _ => log(format!("Reddedilen bağlantı: {}", remote.ip())),
                }
            }
            Some(found) = discovered.recv() => {
                if found.fingerprint_hex == peer_hex {
                    for address in found.addresses.into_iter().rev() {
                        prepend_unique(&mut addresses, address);
                    }
                    if dialer && active.is_none() {
                        next_dial = tokio::time::Instant::now();
                    }
                }
            }
            _ = sleep_until(next_dial), if dialer && active.is_none() => {
                match dial(&addresses, &identity, trust).await {
                    Some((session, address)) => {
                        remember_address(&store, &mut peer, &mut addresses, address.ip())?;
                        log(format!("Bağlandı: {} ({})", session.peer_name, address.ip()));
                        active = activate(session, &state).await;
                    }
                    None => {
                        next_dial = tokio::time::Instant::now() + backoff;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                }
            }
            text = clipboard.local_texts.recv() => {
                let text = text.ok_or_else(|| anyhow!("pano iş parçacığı durdu"))?;
                if text.len() > MAX_TEXT_BYTES {
                    log(format!("Kopya çok büyük ({} bayt), gönderilmedi.", text.len()));
                    continue;
                }
                let Some(event) = state.on_local_text(text) else { continue };
                let (bytes, lamport) = (event.text.len(), event.lamport);
                match active.as_mut() {
                    Some(session) => match session.send(&Message::Clip(event)).await {
                        Ok(()) => log(format!("Gönderildi: {bytes} bayt (lamport {lamport})")),
                        Err(error) => disconnect(&mut active, &mut next_dial, &mut backoff, &error.to_string()),
                    },
                    None => log("Eş bağlı değil; kopya gönderilmedi.".into()),
                }
            }
            message = next_incoming(&mut active) => match message {
                Ok(Message::Clip(event)) => {
                    if state.on_remote(&peer_device_id, &event) {
                        log(format!("Alındı: {} bayt (lamport {})", event.text.len(), event.lamport));
                        clipboard.writes.send(event.text).map_err(|_| anyhow!("pano iş parçacığı durdu"))?;
                    } else {
                        log(format!("Alındı ama uygulanmadı ({} bayt): yerel kopya daha yeni ya da tekrar.", event.text.len()));
                    }
                }
                Ok(Message::Clock { lamport }) => {
                    state.observe_peer_clock(lamport);
                    // Bağlantı ancak eşten ilk geçerli mesaj gelince sağlıklı sayılır;
                    // el sıkışmadan hemen sonra kopan bağlantı beklemeyi sıfırlamaz.
                    backoff = MIN_BACKOFF;
                }
                Ok(Message::Ping { nonce }) => {
                    if let Some(session) = active.as_mut()
                        && let Err(error) = session.send(&Message::Pong { nonce }).await
                    {
                        disconnect(&mut active, &mut next_dial, &mut backoff, &error.to_string());
                    }
                }
                Ok(_) => {}
                Err(error) => disconnect(&mut active, &mut next_dial, &mut backoff, &error.to_string()),
            },
            _ = heartbeat.tick() => {
                let Some(session) = active.as_mut() else { continue };
                if session.last_seen.elapsed() > HEARTBEAT_TIMEOUT {
                    disconnect(&mut active, &mut next_dial, &mut backoff, "eş 15 sn yanıt vermedi");
                } else if let Err(error) = session.send(&Message::Ping { nonce: 0 }).await {
                    disconnect(&mut active, &mut next_dial, &mut backoff, &error.to_string());
                }
            }
        }
    }
}

struct Active {
    writer: WriteHalf<Stream>,
    incoming: mpsc::Receiver<anyhow::Result<Message>>,
    last_seen: Instant,
    reader: JoinHandle<()>,
}

impl Active {
    fn start(session: Session) -> Self {
        let (mut reader_half, writer) = session.split();
        let (sender, incoming) = mpsc::channel(16);
        // read_message select! içinde iptal güvenli değil; ayrı görevde okunur.
        let reader = tokio::spawn(async move {
            loop {
                let message = read_message(&mut reader_half).await;
                let failed = message.is_err();
                if sender.send(message).await.is_err() || failed {
                    return;
                }
            }
        });
        Self { writer, incoming, last_seen: Instant::now(), reader }
    }

    async fn send(&mut self, message: &Message) -> anyhow::Result<()> {
        timeout(HEARTBEAT_TIMEOUT, write_message(&mut self.writer, message)).await.context("gönderim zaman aşımı")?
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        self.reader.abort();
    }
}

/// Oturumu başlatır ve saatimizi bildirir; gönderilemezse oturum kurulmamış sayılır.
async fn activate(session: Session, state: &SyncState) -> Option<Active> {
    let mut active = Active::start(session);
    match active.send(&Message::Clock { lamport: state.clock() }).await {
        Ok(()) => Some(active),
        Err(error) => {
            log(format!("Saat gönderilemedi, bağlantı kapatıldı: {error}"));
            None
        }
    }
}

async fn next_incoming(active: &mut Option<Active>) -> anyhow::Result<Message> {
    let Some(session) = active.as_mut() else {
        return std::future::pending().await;
    };
    let message = session.incoming.recv().await.unwrap_or_else(|| Err(anyhow!("okuma görevi bitti")));
    if message.is_ok() {
        session.last_seen = Instant::now();
    }
    message
}

async fn dial(addresses: &[SocketAddr], identity: &oc_pairing::Identity, trust: Trust) -> Option<(Session, SocketAddr)> {
    for address in addresses {
        if let Ok(Ok(session)) = timeout(DIAL_TIMEOUT, oc_transport::connect(*address, identity, trust)).await {
            return Some((session, *address));
        }
    }
    None
}

fn disconnect(active: &mut Option<Active>, next_dial: &mut tokio::time::Instant, backoff: &mut Duration, reason: &str) {
    if active.take().is_some() {
        log(format!("Bağlantı koptu: {reason}"));
    }
    *next_dial = tokio::time::Instant::now() + *backoff;
    *backoff = (*backoff * 2).min(MAX_BACKOFF);
}

fn remember_address(store: &PeerStore, peer: &mut Peer, addresses: &mut Vec<SocketAddr>, ip: IpAddr) -> anyhow::Result<()> {
    let address = SocketAddr::new(ip, DEFAULT_PORT);
    prepend_unique(addresses, address);
    let text = address.to_string();
    if peer.last_address.as_deref() != Some(text.as_str()) {
        peer.last_address = Some(text);
        store.save(peer)?;
    }
    Ok(())
}

fn prepend_unique(addresses: &mut Vec<SocketAddr>, address: SocketAddr) {
    addresses.retain(|existing| *existing != address);
    addresses.insert(0, address);
}

pub fn log(message: String) {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed().as_secs_f32();
    println!("[{elapsed:7.1} sn] {message}");
}
