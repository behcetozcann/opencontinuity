//! Girişte başlatma. macOS'ta LaunchAgent, Windows'ta Run kaydı, Linux'ta
//! `~/.config/autostart/*.desktop`.
//!
//! tauri-plugin-autostart yerine `auto-launch` doğrudan kullanılır: eklenti
//! LaunchAgent'a `AssociatedBundleIdentifiers` yazmıyor, macOS Sistem
//! Ayarları'ndaki Arka Plan Öğeleri listesinde uygulama adı ve simgesi
//! yerine çıplak "exec" görünüyordu.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use tauri::AppHandle;

/// Girişte otomatik başlatılınca verilen argüman; o zaman pencere gizli kalır.
pub const AUTOSTART_ARG: &str = "--autostart";

/// Yalnız kullanıcı Ayarlar'dan kapatınca yazılır. Kaydın yokluğu kapatma sayılmaz:
/// Windows'ta elle kurulum eski sürümü kaldırırken Run kaydını siler.
const DISABLED_MARKER: &str = "autostart-disabled";
/// Eski sürümlerin işareti; kaydın kaybolmasını "kullanıcı kapattı" sanıyordu.
const OLD_MARKER: &str = "autostart-initialized";

static LAUNCHER: OnceLock<Result<AutoLaunch, String>> = OnceLock::new();
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

fn launcher(app: &AppHandle) -> Result<&'static AutoLaunch, String> {
    LAUNCHER.get_or_init(|| build(app)).as_ref().map_err(Clone::clone)
}

fn build(app: &AppHandle) -> Result<AutoLaunch, String> {
    let exe = match appimage_path() {
        Some(path) => path,
        None => std::env::current_exe().and_then(|path| path.canonicalize()).map_err(|error| error.to_string())?,
    };
    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(&app.package_info().name).set_app_path(&launch_path(&exe)).set_args(&[AUTOSTART_ARG]);
    // Kurulum kullanıcı başına; varsayılan kip yönetici yetkisi varsa HKLM'ye yazıyordu.
    #[cfg(windows)]
    builder.set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
    #[cfg(target_os = "macos")]
    builder
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .set_bundle_identifiers(&[app.config().identifier.as_str()]);
    builder.build().map_err(|error| error.to_string())
}

/// AppImage çalışırken `current_exe` her açılışta değişen geçici bağlama
/// noktasını gösterir; girişte açılacak olan AppImage dosyasının kendisidir.
fn appimage_path() -> Option<PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    std::env::var_os("APPIMAGE").map(PathBuf::from)
}

/// Windows'ta `canonicalize` `\\?\C:\...` biçimi verir; Run kaydında bu önekli,
/// tırnaksız yolla girişte uygulama açılmadı. Önek atılır, boşluklu yol için tırnaklanır.
fn launch_path(exe: &Path) -> String {
    let path = exe.display().to_string();
    if cfg!(windows) {
        format!("\"{}\"", path.strip_prefix(r"\\?\").unwrap_or(&path))
    } else if cfg!(target_os = "linux") && path.contains(' ') {
        // .desktop `Exec=` satırında boşluklu yol tırnaklanmazsa argümanlara bölünür.
        format!("\"{path}\"")
    } else {
        path
    }
}

pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    launcher(app)?.is_enabled().map_err(|error| error.to_string())
}

/// Açılışta: kullanıcı kapatmadıysa kayıt her seferinde yeniden yazılır
/// (silinmiş ya da eski yolu gösteriyor olabilir).
pub fn sync(app: &AppHandle, config_dir: &Path) {
    let _ = CONFIG_DIR.set(config_dir.to_path_buf());
    let _ = std::fs::remove_file(config_dir.join(OLD_MARKER));
    // Geliştirme derlemesi girişe kaydedilmez; kayıt target/ altındaki binary'yi gösterirdi.
    if cfg!(debug_assertions) {
        return;
    }
    if config_dir.join(DISABLED_MARKER).exists() {
        crate::diagnostics::write("Girişte başlatma: kullanıcı kapattı.");
        return;
    }
    let was_enabled = is_enabled(app).unwrap_or(false);
    match set_enabled(app, true) {
        Ok(()) if was_enabled => crate::diagnostics::write("Girişte başlatma: açık."),
        Ok(()) => crate::diagnostics::write("Girişte başlatma: kayıt yoktu, yeniden yazıldı."),
        Err(error) => crate::diagnostics::write(&format!("Girişte başlatma açılamadı: {error}")),
    }
}

pub fn set_by_user(app: &AppHandle, enabled: bool) -> Result<(), String> {
    set_enabled(app, enabled)?;
    let Some(dir) = CONFIG_DIR.get() else { return Ok(()) };
    let marker = dir.join(DISABLED_MARKER);
    let result = if enabled { std::fs::remove_file(marker).or_else(ignore_missing) } else { std::fs::write(marker, b"") };
    result.map_err(|error| error.to_string())
}

fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = launcher(app)?;
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|error| error.to_string())
}

fn ignore_missing(error: std::io::Error) -> std::io::Result<()> {
    if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) }
}
