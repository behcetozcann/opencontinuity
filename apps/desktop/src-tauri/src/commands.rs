//! Arayüzün çağırdığı komutlar; motor komutlarına çevrilir.

use oc_engine::{ClipboardSource, Command, Engine, EngineConfig};
use tauri::{AppHandle, State};

use crate::{AppState, View, autostart, diagnostics, spawn_engine, update_view, updater};

#[tauri::command]
pub fn get_view(state: State<'_, AppState>) -> View {
    state.view.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

#[tauri::command]
pub fn start_pairing(app: AppHandle, state: State<'_, AppState>, address: Option<String>) {
    update_view(&app, |view| view.pairing_error = None);
    let address = address.map(|address| address.trim().to_owned()).filter(|address| !address.is_empty());
    state.engine().send(Command::StartPairing { address });
}

#[tauri::command]
pub fn confirm_code(state: State<'_, AppState>, accepted: bool) {
    state.engine().send(Command::ConfirmCode(accepted));
}

#[tauri::command]
pub fn cancel_pairing(state: State<'_, AppState>) {
    state.engine().send(Command::CancelPairing);
}

#[tauri::command]
pub fn set_paused(state: State<'_, AppState>, paused: bool) {
    state.engine().send(if paused { Command::Pause } else { Command::Resume });
}

#[tauri::command]
pub fn unpair(state: State<'_, AppState>, fingerprint_prefix: String) {
    state.engine().send(Command::Unpair { fingerprint_prefix });
}

#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> Result<bool, String> {
    autostart::is_enabled(&app)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    autostart::set_by_user(&app, enabled)
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Motor durduysa (ör. port kullanımdaydı) yeniden başlatır.
#[tauri::command]
pub fn restart_engine(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if get_view(state.clone()).engine_error.is_none() {
        return Ok(());
    }
    let config = EngineConfig::system().map_err(|error| format!("{error:#}"))?;
    let (engine, handle, events) = Engine::new(config, ClipboardSource::System);
    *state.handle.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = handle;
    diagnostics::write("Motor yeniden başlatılıyor");
    update_view(&app, |view| view.engine_error = None);
    spawn_engine(app.clone(), engine, events);
    Ok(())
}

#[tauri::command]
pub async fn check_update(app: AppHandle) {
    updater::check(&app).await;
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}

/// macOS Sistem Ayarları → Gizlilik ve Güvenlik → Yerel Ağ.
#[tauri::command]
pub fn open_local_network_settings() -> Result<(), String> {
    open_external("x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork")
}

#[tauri::command]
pub fn open_project_page() -> Result<(), String> {
    open_external("https://github.com/behcetozcann/opencontinuity")
}

#[tauri::command]
pub fn open_files_dir() -> Result<(), String> {
    let dir = oc_engine::config::files_dir().map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    open_external(&dir.to_string_lossy())
}

/// Son aktarımlardaki gelen dosyaların klasörünü açar; yol arayüzden değil kayıttan alınır.
#[tauri::command]
pub fn reveal_transfer(state: State<'_, AppState>, at_ms: u64) -> Result<(), String> {
    let folder = state.view.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).saved_folder(at_ms);
    let folder = folder.ok_or("bu aktarımın klasörü yok")?;
    if !folder.exists() {
        return Err("klasör artık yok (silinmiş ya da taşınmış)".into());
    }
    open_external(&folder.to_string_lossy())
}

/// Yalnız bu dosyadaki sabit adresler ve uygulamanın kaydettiği klasörler açılır.
fn open_external(target: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(target).spawn();
    #[cfg(windows)]
    let result = std::process::Command::new("explorer").arg(target).spawn();
    #[cfg(not(any(target_os = "macos", windows)))]
    let result = std::process::Command::new("xdg-open").arg(target).spawn();
    result.map(|_| ()).map_err(|error| format!("açılamadı: {error}"))
}
