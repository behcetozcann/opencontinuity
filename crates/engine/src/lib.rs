//! Senkron motoru. Kabuk (CLI ya da masaüstü) [`Engine::new`] ile motoru
//! kurar, [`Engine::run`]'ı kendi tokio çalışma ortamında başlatır, durumu
//! [`Event`] kanalından okur ve [`Handle`] ile komut gönderir.
//!
//! Durumlar: eş yoksa boşta (port kapalı) → `StartPairing` ile eşleştirme →
//! eş kaydedilince senkron döngüsü (gruptaki bütün eşlerle) → son eş
//! kaldırılınca tekrar boşta. Senkrondayken `StartPairing` gruba yeni cihaz
//! ekler; eşleştirme süresince senkron durur.

mod advertise;
mod clipboard_thread;
pub mod config;
mod dial;
mod events;
mod files;
mod group;
mod link;
mod pairing;
mod pairing_host;
mod sync_loop;
mod transfer;
mod writer;

use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use oc_pairing::{Identity, PeerStore};
use tokio::sync::mpsc;

pub use advertise::{DiscoveryMode, ExternalDiscovery};
pub use events::{Event, PairingStage, PeerSummary, Snapshot};
pub use oc_discovery::Found;
pub use oc_protocol::ContentKind;

#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout: Duration,
    pub handshake_timeout: Duration,
    pub dial_timeout: Duration,
    pub min_backoff: Duration,
    pub max_backoff: Duration,
    /// Bağlantı kopukken yapılan son kopya bu süre içinde bağlanılırsa gönderilir.
    pub offline_copy_max_age: Duration,
    /// Bağlantıyı başlatmayan taraf, bu süre içinde gelen bağlantı olmazsa
    /// kendisi arar (karşı tarafın güvenlik duvarı gelen bağlantıyı engelliyorsa).
    pub fallback_delay: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            heartbeat_timeout: Duration::from_secs(15),
            handshake_timeout: Duration::from_secs(5),
            dial_timeout: Duration::from_secs(3),
            min_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            offline_copy_max_age: Duration::from_secs(120),
            fallback_delay: Duration::from_secs(5),
        }
    }
}

pub struct EngineConfig {
    pub config_dir: PathBuf,
    pub device_name: String,
    pub port: u16,
    /// Cihaz yayını ve arama; testlerde kapalı, iOS'ta kabuğun Bonjour'u.
    pub discovery: DiscoveryMode,
    pub timing: Timing,
    /// Gelen dosyaların kaydedildiği klasör (İndirilenler/OpenContinuity).
    pub files_dir: PathBuf,
}

impl EngineConfig {
    /// Kullanıcının config dizini, bilgisayar adı ve varsayılan port.
    pub fn system() -> anyhow::Result<Self> {
        Ok(Self {
            config_dir: config::default_dir()?,
            device_name: config::device_name(),
            port: config::port()?,
            discovery: DiscoveryMode::Mdns,
            timing: Timing::default(),
            files_dir: config::files_dir()?,
        })
    }
}

/// Bu bilgisayarın panosundaki içerik. Dosyalar yerel yollardır; karşıya
/// liste + içerik olarak gider, gelenler `files_dir` altındaki yollarla yazılır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    Text(String),
    Png(Vec<u8>),
    Files(Vec<PathBuf>),
}

/// Panoyla konuşan kanallar; testler sahte pano için kendisi kurar.
pub struct ClipboardPort {
    /// Yerelde kopyalananlar.
    pub local_copies: mpsc::Receiver<Clip>,
    /// Panoya yazılacak uzak içerik.
    pub writes: std_mpsc::Sender<Clip>,
}

pub enum ClipboardSource {
    System,
    Port(ClipboardPort),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `address` yoksa aynı ağda eşleştirme modundaki cihaz aranır ve beklenir.
    StartPairing { address: Option<String> },
    ConfirmCode(bool),
    CancelPairing,
    Pause,
    Resume,
    /// Gruptan bir cihazı kaldırır (`PeerSummary::fingerprint_prefix`).
    Unpair { fingerprint_prefix: String },
    Shutdown,
}

#[derive(Clone)]
pub struct Handle {
    commands: mpsc::UnboundedSender<Command>,
}

impl Handle {
    /// Motor durduysa komut sessizce düşer.
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

pub struct Engine {
    config: EngineConfig,
    clipboard: ClipboardSource,
    commands: mpsc::UnboundedReceiver<Command>,
    events: events::Emitter,
}

impl Engine {
    pub fn new(config: EngineConfig, clipboard: ClipboardSource) -> (Self, Handle, mpsc::UnboundedReceiver<Event>) {
        let (command_sender, commands) = mpsc::unbounded_channel();
        let (event_sender, events) = mpsc::unbounded_channel();
        let engine = Self { config, clipboard, commands, events: events::Emitter::new(event_sender) };
        (engine, Handle { commands: command_sender }, events)
    }

    /// `Shutdown` gelene ya da tüm `Handle`'lar düşene kadar çalışır.
    pub async fn run(self) -> anyhow::Result<()> {
        let identity = Identity::load_or_create(&self.config.config_dir, &self.config.device_name)?;
        let clipboard = match self.clipboard {
            ClipboardSource::System => clipboard_thread::spawn(self.events.sender())?,
            ClipboardSource::Port(port) => port,
        };
        let mut context = Context {
            store: PeerStore::new(&self.config.config_dir),
            config: self.config,
            identity,
            clipboard,
            commands: self.commands,
            events: self.events,
        };
        let fingerprint_prefix = hex::encode(context.identity.fingerprint())[..16].to_owned();
        let device_name = context.identity.name.clone();
        context.events.update(|snapshot| {
            snapshot.device_name = device_name;
            snapshot.fingerprint_prefix = fingerprint_prefix;
        });
        let peers = context.store.load_all()?;
        sync_loop::run(&mut context, peers).await
    }
}

pub(crate) struct Context {
    config: EngineConfig,
    identity: Identity,
    store: PeerStore,
    clipboard: ClipboardPort,
    commands: mpsc::UnboundedReceiver<Command>,
    events: events::Emitter,
}

/// mDNS kapalıysa hiç dönmez.
pub(crate) async fn next_found(discovered: &mut Option<mpsc::Receiver<Found>>) -> Option<Found> {
    match discovered {
        Some(receiver) => receiver.recv().await,
        None => std::future::pending().await,
    }
}
