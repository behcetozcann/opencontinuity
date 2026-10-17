//! Arayüzün çağırdığı komutlar; motor komutlarına çevrilir.

use oc_engine::Command;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{AppState, View, update_view};

#[tauri::command]
pub fn get_view(state: State<'_, AppState>) -> View {
    state.view.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

#[tauri::command]
pub fn start_pairing(app: AppHandle, state: State<'_, AppState>, address: Option<String>) {
    update_view(&app, |view| view.pairing_error = None);
    let address = address.map(|address| address.trim().to_owned()).filter(|address| !address.is_empty());
    state.handle.send(Command::StartPairing { address });
}

#[tauri::command]
pub fn confirm_code(state: State<'_, AppState>, accepted: bool) {
    state.handle.send(Command::ConfirmCode(accepted));
}

#[tauri::command]
pub fn cancel_pairing(state: State<'_, AppState>) {
    state.handle.send(Command::CancelPairing);
}

#[tauri::command]
pub fn set_paused(state: State<'_, AppState>, paused: bool) {
    state.handle.send(if paused { Command::Pause } else { Command::Resume });
}

#[tauri::command]
pub fn unpair(state: State<'_, AppState>) {
    state.handle.send(Command::Unpair);
}

#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    result.map_err(|error| error.to_string())
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}
