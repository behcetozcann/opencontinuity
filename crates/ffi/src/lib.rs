//! iOS kabuğunun motoru kullandığı yüzey. Swift `OcEngine`'i başlatır,
//! durumu `EngineListener` ile alır; pano ve Bonjour Swift'tedir, motor
//! bunları `ClipboardPort` ve `DiscoveryMode::External` ile görür.

mod records;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc as std_mpsc;

use oc_engine::{
    Clip, ClipboardPort, ClipboardSource, Command, DiscoveryMode, Engine, EngineConfig, Event, ExternalDiscovery, Found,
    Handle, Timing,
};
use tokio::sync::mpsc;

pub use records::{ClipRecord, ContentKindRecord, EventRecord, PairingRecord, PeerRecord, SnapshotRecord};

uniffi::setup_scaffolding!();

#[derive(uniffi::Record)]
pub struct StartConfig {
    /// Kimlik ve eş listesi (uygulamanın Application Support dizini).
    pub config_dir: String,
    /// Gelen dosyalar (Documents altında; Dosyalar uygulamasında görünür).
    pub files_dir: String,
    pub device_name: String,
    pub port: u16,
}

/// Swift tarafı uygular; çağrılar motorun iş parçacıklarından gelir.
#[uniffi::export(with_foreign)]
pub trait EngineListener: Send + Sync {
    fn on_state(&self, state: SnapshotRecord);
    fn on_event(&self, event: EventRecord);
    /// Eşten gelen içerik panoya yazılmalı (dosyalarda kaydedilen yollar).
    fn write_clipboard(&self, clip: ClipRecord);
    /// Bonjour TXT kaydındaki `pair` işareti.
    fn set_advertise_pairing(&self, pairing: bool);
    /// Motor durdu; `error` varsa başlatılamadı ya da çalışırken düştü.
    fn on_stopped(&self, error: Option<String>);
}

/// Bu cihazın tam parmak izi (hex); Bonjour TXT `fp` kaydı için. Kimlik yoksa
/// oluşturulur, motor aynı dosyayı kullanır. Okunamazsa `None`.
#[uniffi::export]
pub fn device_fingerprint(config_dir: String, device_name: String) -> Option<String> {
    let identity = oc_pairing::Identity::load_or_create(std::path::Path::new(&config_dir), &device_name).ok()?;
    Some(hex::encode(identity.fingerprint()))
}

#[derive(uniffi::Object)]
pub struct OcEngine {
    handle: Handle,
    copies: mpsc::Sender<Clip>,
    found: mpsc::Sender<Found>,
}

#[uniffi::export]
impl OcEngine {
    #[uniffi::constructor]
    pub fn start(config: StartConfig, listener: Arc<dyn EngineListener>) -> Arc<Self> {
        let (copies, local_copies) = mpsc::channel(8);
        let (writes, written) = std_mpsc::channel();
        let (found, found_receiver) = mpsc::channel(16);
        let advertise_listener = listener.clone();
        let engine_config = EngineConfig {
            config_dir: PathBuf::from(config.config_dir),
            device_name: config.device_name,
            port: config.port,
            discovery: DiscoveryMode::External(ExternalDiscovery {
                found: found_receiver,
                set_pairing: Box::new(move |pairing| advertise_listener.set_advertise_pairing(pairing)),
            }),
            timing: Timing::default(),
            files_dir: PathBuf::from(config.files_dir),
        };
        let (engine, handle, events) =
            Engine::new(engine_config, ClipboardSource::Port(ClipboardPort { local_copies, writes }));
        spawn_clipboard_writer(written, listener.clone());
        spawn_engine(engine, events, listener);
        Arc::new(Self { handle, copies, found })
    }

    /// `address` yoksa ağda eşleştirme modundaki cihaz aranır.
    pub fn start_pairing(&self, address: Option<String>) {
        self.handle.send(Command::StartPairing { address });
    }

    pub fn confirm_code(&self, accepted: bool) {
        self.handle.send(Command::ConfirmCode(accepted));
    }

    pub fn cancel_pairing(&self) {
        self.handle.send(Command::CancelPairing);
    }

    pub fn unpair(&self, fingerprint_prefix: String) {
        self.handle.send(Command::Unpair { fingerprint_prefix });
    }

    pub fn pause(&self) {
        self.handle.send(Command::Pause);
    }

    pub fn resume(&self) {
        self.handle.send(Command::Resume);
    }

    pub fn shutdown(&self) {
        self.handle.send(Command::Shutdown);
    }

    /// Bu cihazda kopyalanan içerik; kuyruk doluysa (motor meşgul) düşer ve `false` döner.
    pub fn push_clip(&self, clip: ClipRecord) -> bool {
        let clip = match clip {
            ClipRecord::Text { text } => Clip::Text(text),
            ClipRecord::Png { data } => Clip::Png(data),
            ClipRecord::Files { paths } => Clip::Files(paths.into_iter().map(PathBuf::from).collect()),
        };
        self.copies.try_send(clip).is_ok()
    }

    /// Bonjour'da çözümlenen cihaz; `addresses` "ip:port" biçiminde.
    pub fn bonjour_found(&self, name: String, fingerprint_hex: String, pairing: bool, addresses: Vec<String>) {
        let addresses: Vec<SocketAddr> = addresses.iter().filter_map(|address| address.parse().ok()).collect();
        if addresses.is_empty() {
            return;
        }
        let _ = self.found.try_send(Found { name, fingerprint_hex, pairing, addresses });
    }
}

fn spawn_clipboard_writer(written: std_mpsc::Receiver<Clip>, listener: Arc<dyn EngineListener>) {
    std::thread::spawn(move || {
        while let Ok(clip) = written.recv() {
            listener.write_clipboard(clip.into());
        }
    });
}

fn spawn_engine(engine: Engine, mut events: mpsc::UnboundedReceiver<Event>, listener: Arc<dyn EngineListener>) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => return listener.on_stopped(Some(format!("çalışma ortamı kurulamadı: {error}"))),
        };
        let event_listener = listener.clone();
        runtime.spawn(async move {
            while let Some(event) = events.recv().await {
                match event {
                    Event::State(snapshot) => event_listener.on_state(snapshot.into()),
                    event => {
                        if let Some(record) = EventRecord::from_event(event) {
                            event_listener.on_event(record);
                        }
                    }
                }
            }
        });
        let result = runtime.block_on(engine.run());
        listener.on_stopped(result.err().map(|error| format!("{error:#}")));
    });
}
