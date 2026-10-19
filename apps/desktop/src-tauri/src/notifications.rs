//! Başka cihazdan dosya ya da görsel gelince sesli bildirim (AirDrop gibi).
//! Metin sessiz kalır: her kopyada bildirim rahatsız eder.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use oc_engine::ContentKind;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Yalnız kullanıcı Ayarlar'dan kapatınca yazılır; varsayılan açık.
const DISABLED_MARKER: &str = "notifications-disabled";

/// Sistem sesi: macOS `/System/Library/Sounds`, Windows toast, Linux freedesktop adı.
#[cfg(target_os = "macos")]
const SOUND: &str = "Glass";
#[cfg(target_os = "windows")]
const SOUND: &str = "Default";
#[cfg(target_os = "linux")]
const SOUND: &str = "message-new-instant";

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init(config_dir: &Path) {
    let _ = CONFIG_DIR.set(config_dir.to_path_buf());
}

pub fn is_enabled() -> bool {
    CONFIG_DIR.get().is_none_or(|dir| !dir.join(DISABLED_MARKER).exists())
}

pub fn set_by_user(enabled: bool) -> Result<(), String> {
    let Some(dir) = CONFIG_DIR.get() else { return Ok(()) };
    let marker = dir.join(DISABLED_MARKER);
    let result = if enabled {
        std::fs::remove_file(marker).or_else(|error| if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) })
    } else {
        std::fs::write(marker, b"")
    };
    result.map_err(|error| error.to_string())
}

pub fn received(app: &AppHandle, content: ContentKind, items: usize, from: &str) {
    if !is_enabled() {
        return;
    }
    let body = match content {
        ContentKind::Files if items == 1 => "Bir dosya gönderdi · İndirilenler › OpenContinuity".to_owned(),
        ContentKind::Files => format!("{items} öğe gönderdi · İndirilenler › OpenContinuity"),
        ContentKind::Image => "Görsel gönderdi · panoya kopyalandı".to_owned(),
        ContentKind::Text => return,
    };
    if let Err(error) = app.notification().builder().title(from).body(body).sound(SOUND).show() {
        crate::diagnostics::write(&format!("Bildirim gösterilemedi: {error}"));
    }
}
