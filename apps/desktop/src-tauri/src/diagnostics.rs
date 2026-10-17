//! Teşhis kaydı: `<config>/oc.log` (önceki oturum `oc.old.log`). Gerçek cihaz
//! testinde kullanıcı bu dosyayı gönderir. Pano içeriği yazılmaz; motor
//! olayları zaten yalnız bayt sayısı, lamport ve hash öneki taşır.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use oc_engine::Event;

static LOG: OnceLock<Mutex<File>> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();

pub fn open(config_dir: &Path) {
    let path = config_dir.join("oc.log");
    let _ = fs::create_dir_all(config_dir);
    let _ = fs::rename(&path, config_dir.join("oc.old.log"));
    if let Ok(file) = File::create(path) {
        let _ = LOG.set(Mutex::new(file));
    }
    STARTED.get_or_init(Instant::now);
    write(&format!("OpenContinuity {} ({}-{})", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH));
}

pub fn write(message: &str) {
    let Some(file) = LOG.get() else { return };
    let elapsed = STARTED.get_or_init(Instant::now).elapsed().as_secs_f32();
    let mut file = file.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _ = writeln!(file, "[{elapsed:8.1} sn] {message}");
}

pub fn record(event: &Event) {
    let line = match event {
        Event::State(snapshot) => format!(
            "Durum: eş={} bağlı={} duraklatıldı={} eşleştirme={}",
            snapshot.peer.as_ref().map_or("yok", |peer| peer.name.as_str()),
            snapshot.connected_ip.as_deref().unwrap_or("hayır"),
            snapshot.paused,
            match &snapshot.pairing {
                None => "yok",
                Some(oc_engine::PairingStage::Searching { .. }) => "aranıyor",
                Some(oc_engine::PairingStage::Confirm { .. }) => "kod onayı",
                Some(oc_engine::PairingStage::WaitingPeer { .. }) => "eş onayı bekleniyor",
            }
        ),
        Event::Sent { bytes, lamport } => format!("Gönderildi: {bytes} bayt (lamport {lamport})"),
        Event::Received { bytes, lamport } => format!("Alındı: {bytes} bayt (lamport {lamport})"),
        Event::Skipped { reason } => reason.clone(),
        Event::Disconnected { reason } => format!("Bağlantı koptu: {reason}"),
        Event::Paired { peer } => format!("Eşleşti: {peer}"),
        Event::PairingFailed { reason } => format!("Eşleştirme başarısız: {reason}"),
        Event::Diagnostic { message } => message.clone(),
    };
    write(&line);
}
