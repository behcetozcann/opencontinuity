//! Teşhis kaydı: `<config>/oc.log` (önceki oturum `oc.old.log`). Gerçek cihaz
//! testinde kullanıcı bu dosyayı gönderir. Pano içeriği yazılmaz; motor
//! olayları zaten yalnız bayt sayısı, lamport ve hash öneki taşır.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use oc_engine::Event;

const LOG_FILE: &str = "oc.log";
const OLD_LOG_FILE: &str = "oc.old.log";
/// Arayüzde gösterilen/kopyalanan en fazla satır.
const MAX_SHARED_LINES: usize = 2000;

static LOG: OnceLock<Mutex<File>> = OnceLock::new();
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();

pub fn open(config_dir: &Path) {
    let path = config_dir.join(LOG_FILE);
    let _ = fs::create_dir_all(config_dir);
    let _ = fs::rename(&path, config_dir.join(OLD_LOG_FILE));
    let _ = LOG_DIR.set(config_dir.to_path_buf());
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

/// Önceki ve bu oturumun kaydı, başlık bilgisiyle; çok uzunsa son satırlar.
#[tauri::command]
pub fn read_log() -> Result<String, String> {
    let dir = LOG_DIR.get().ok_or("kayıt dizini yok")?;
    if let Some(file) = LOG.get() {
        let _ = file.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).flush();
    }
    let mut text = format!(
        "OpenContinuity {} — {} {} — cihaz: {}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        oc_engine::config::device_name()
    );
    for (title, name) in [("Önceki oturum", OLD_LOG_FILE), ("Bu oturum", LOG_FILE)] {
        if let Ok(content) = fs::read_to_string(dir.join(name)) {
            text.push_str(&format!("\n===== {title} ({name}) =====\n{content}"));
        }
    }
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= MAX_SHARED_LINES {
        return Ok(text);
    }
    let skipped = lines.len() - MAX_SHARED_LINES;
    Ok(format!("{}\n[... ilk {skipped} satır atlandı ...]\n{}", lines[0], lines[skipped + 1..].join("\n")))
}

/// Kaydı Masaüstüne tek dosya olarak yazar ve yolunu döner.
#[tauri::command]
pub fn save_log_to_desktop() -> Result<String, String> {
    let desktop = directories::UserDirs::new()
        .and_then(|dirs| dirs.desktop_dir().map(Path::to_path_buf))
        .ok_or("Masaüstü klasörü bulunamadı")?;
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0);
    let path = desktop.join(format!("OpenContinuity-kayit-{}-{seconds}.txt", oc_engine::config::device_name()));
    fs::write(&path, read_log()?).map_err(|error| format!("kaydedilemedi: {error}"))?;
    Ok(path.display().to_string())
}

/// Kayıt dosyasını Finder / Dosya Gezgini'nde seçili gösterir.
#[tauri::command]
pub fn reveal_log() -> Result<(), String> {
    let path = LOG_DIR.get().ok_or("kayıt dizini yok")?.join(LOG_FILE);
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg("-R").arg(&path).spawn();
    #[cfg(windows)]
    let result = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    #[cfg(not(any(target_os = "macos", windows)))]
    let result = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(&path)).spawn();
    result.map(|_| ()).map_err(|error| format!("klasör açılamadı: {error}"))
}
