//! Senkron motoru. Kabuk (CLI ya da masaüstü) [`Engine::new`] ile motoru
//! kurar, [`Engine::run`]'ı kendi tokio çalışma ortamında başlatır, durumu
//! [`Event`] kanalından okur ve [`Handle`] ile komut gönderir.
//!
//! Durumlar: eş yoksa boşta (port kapalı) → `StartPairing` ile eşleştirme →
//! eş kaydedilince senkron döngüsü → `Unpair` ile tekrar boşta.

mod clipboard_thread;
pub mod config;
mod events;
mod link;
mod pairing;
mod sync_loop;

use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use anyhow::anyhow;
use oc_discovery::Found;
use oc_pairing::{Identity, PeerStore};
use tokio::sync::mpsc;

pub use events::{Event, PairingStage, PeerSummary, Snapshot};
pub use oc_protocol::{ContentKind, Payload};

#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout: Duration,
    pub handshake_timeout: Duration,
    pub dial_timeout: Duration,
    pub min_backoff: Duration,
    pub max_backoff: Duration,
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
        }
    }
}

pub struct EngineConfig {
    pub config_dir: PathBuf,
    pub device_name: String,
    pub port: u16,
    /// mDNS yayını ve arama (testlerde kapalı).
    pub discovery: bool,
    pub timing: Timing,
}

impl EngineConfig {
    /// Kullanıcının config dizini, bilgisayar adı ve varsayılan port.
    pub fn system() -> anyhow::Result<Self> {
        Ok(Self {
            config_dir: config::default_dir()?,
            device_name: config::device_name(),
            port: config::port()?,
            discovery: true,
            timing: Timing::default(),
        })
    }
}

/// Panoyla konuşan kanallar; testler sahte pano için kendisi kurar.
pub struct ClipboardPort {
    /// Yerelde kopyalanan metin ve görseller.
    pub local_copies: mpsc::Receiver<Payload>,
    /// Panoya yazılacak uzak içerik.
    pub writes: std_mpsc::Sender<Payload>,
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
    Unpair,
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
        loop {
            let flow = match context.store.load()? {
                Some(peer) => sync_loop::run(&mut context, peer).await?,
                None => idle(&mut context).await?,
            };
            if flow == Flow::Shutdown {
                return Ok(());
            }
        }
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    /// Eş kaydı değişmiş olabilir; durum yeniden belirlenir.
    Continue,
    Shutdown,
}

/// Eş yokken: port kapalı, eşleştirme komutu beklenir.
async fn idle(context: &mut Context) -> anyhow::Result<Flow> {
    context.events.update(|snapshot| {
        snapshot.peer = None;
        snapshot.connected_ip = None;
        snapshot.pairing = None;
    });
    loop {
        tokio::select! {
            command = context.commands.recv() => match command {
                None | Some(Command::Shutdown) => return Ok(Flow::Shutdown),
                Some(Command::StartPairing { address }) => return pairing::run(context, address).await,
                Some(Command::Pause) => context.events.update(|snapshot| snapshot.paused = true),
                Some(Command::Resume) => context.events.update(|snapshot| snapshot.paused = false),
                Some(_) => {}
            },
            // Eş yokken kopyalar okunup atılır; kanal dolup pano iş parçacığı tıkanmasın.
            copy = context.clipboard.local_copies.recv() => {
                copy.ok_or_else(|| anyhow!("pano iş parçacığı durdu"))?;
            }
        }
    }
}

/// mDNS kapalıysa hiç dönmez.
pub(crate) async fn next_found(discovered: &mut Option<mpsc::Receiver<Found>>) -> Option<Found> {
    match discovered {
        Some(receiver) => receiver.recv().await,
        None => std::future::pending().await,
    }
}
