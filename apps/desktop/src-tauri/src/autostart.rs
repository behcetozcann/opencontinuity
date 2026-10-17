//! Girişte başlatma. macOS'ta LaunchAgent, Windows'ta Run kaydı.
//!
//! tauri-plugin-autostart yerine `auto-launch` doğrudan kullanılır: eklenti
//! LaunchAgent'a `AssociatedBundleIdentifiers` yazmıyor, macOS Sistem
//! Ayarları'ndaki Arka Plan Öğeleri listesinde uygulama adı ve simgesi
//! yerine çıplak "exec" görünüyordu.

use std::sync::OnceLock;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use tauri::AppHandle;

/// Girişte otomatik başlatılınca verilen argüman; o zaman pencere gizli kalır.
pub const AUTOSTART_ARG: &str = "--autostart";

static LAUNCHER: OnceLock<Result<AutoLaunch, String>> = OnceLock::new();

fn launcher(app: &AppHandle) -> Result<&'static AutoLaunch, String> {
    LAUNCHER.get_or_init(|| build(app)).as_ref().map_err(Clone::clone)
}

fn build(app: &AppHandle) -> Result<AutoLaunch, String> {
    let exe = std::env::current_exe().and_then(|path| path.canonicalize()).map_err(|error| error.to_string())?;
    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(&app.package_info().name).set_app_path(&exe.display().to_string()).set_args(&[AUTOSTART_ARG]);
    #[cfg(target_os = "macos")]
    builder
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .set_bundle_identifiers(&[app.config().identifier.as_str()]);
    builder.build().map_err(|error| error.to_string())
}

pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    launcher(app)?.is_enabled().map_err(|error| error.to_string())
}

pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = launcher(app)?;
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|error| error.to_string())
}
