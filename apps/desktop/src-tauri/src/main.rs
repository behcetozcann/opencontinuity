// Windows sürüm derlemesinde konsol penceresi açılmasın.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod diagnostics;
mod tray;

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use oc_engine::{ClipboardSource, Engine, EngineConfig, Event, Handle, Snapshot};
use oc_pairing::PeerStore;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tokio::sync::mpsc::UnboundedReceiver;

/// İlk açılışta girişte başlatmayı bir kez açtığımızı hatırlar; kullanıcı kapatırsa saygı duyulur.
const AUTOSTART_MARKER: &str = "autostart-initialized";

#[derive(Debug, Clone, Serialize)]
pub struct Transfer {
    direction: &'static str,
    bytes: usize,
    at_ms: u64,
}

impl Transfer {
    fn now(direction: &'static str, bytes: usize) -> Self {
        let at_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0);
        Self { direction, bytes, at_ms }
    }
}

/// Arayüzün çizdiği her şey; her değişimde `view` olayıyla tamamı gönderilir.
#[derive(Debug, Clone, Default, Serialize)]
pub struct View {
    snapshot: Snapshot,
    last_transfer: Option<Transfer>,
    pairing_error: Option<String>,
    /// Motor durduysa (ör. port kullanımda) nedeni.
    engine_error: Option<String>,
}

pub struct AppState {
    handle: Handle,
    view: Mutex<View>,
}

fn main() {
    let mut builder = tauri::Builder::default();
    // OC_CONFIG_DIR ile aynı makinede ikinci örnek açılabilsin (eşleştirme denemesi).
    if std::env::var_os("OC_CONFIG_DIR").is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)));
    }
    let app = builder
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::start_pairing,
            commands::confirm_code,
            commands::cancel_pairing,
            commands::set_paused,
            commands::unpair,
            commands::autostart_enabled,
            commands::set_autostart,
            commands::app_version,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let config = EngineConfig::system()?;
            let unpaired = PeerStore::new(&config.config_dir).load()?.is_none();
            enable_autostart_once(app.handle(), &config.config_dir);
            diagnostics::open(&config.config_dir);
            let (engine, handle, events) = Engine::new(config, ClipboardSource::System);
            app.manage(AppState { handle, view: Mutex::new(View::default()) });
            tray::create(app.handle())?;
            spawn_engine(app.handle().clone(), engine, events);
            if unpaired {
                show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Pencereyi kapatmak uygulamayı kapatmaz; menü çubuğunda/tepside çalışmayı sürdürür.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("OpenContinuity başlatılamadı");
    app.run(|_app, event| {
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn spawn_engine(app: AppHandle, engine: Engine, mut events: UnboundedReceiver<Event>) {
    let app_for_errors = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = engine.run().await {
            let message = format!("{error:#}");
            diagnostics::write(&format!("Motor durdu: {message}"));
            update_view(&app_for_errors, |view| view.engine_error = Some(message));
            show_window(&app_for_errors);
        }
    });
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            diagnostics::record(&event);
            apply_event(&app, event);
        }
    });
}

fn apply_event(app: &AppHandle, event: Event) {
    match event {
        Event::State(snapshot) => update_view(app, |view| view.snapshot = snapshot),
        Event::Sent { bytes, .. } => update_view(app, |view| view.last_transfer = Some(Transfer::now("sent", bytes))),
        Event::Received { bytes, .. } => update_view(app, |view| view.last_transfer = Some(Transfer::now("received", bytes))),
        Event::PairingFailed { reason } => update_view(app, |view| view.pairing_error = Some(reason)),
        Event::Paired { .. } => update_view(app, |view| view.pairing_error = None),
        Event::Skipped { .. } | Event::Disconnected { .. } | Event::Diagnostic { .. } => {}
    }
}

pub fn update_view(app: &AppHandle, change: impl FnOnce(&mut View)) {
    let state = app.state::<AppState>();
    let view = {
        let mut view = state.view.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        change(&mut view);
        view.clone()
    };
    let _ = app.emit("view", &view);
    tray::refresh(app, &view);
}

fn enable_autostart_once(app: &AppHandle, config_dir: &std::path::Path) {
    let marker = config_dir.join(AUTOSTART_MARKER);
    // Geliştirme derlemesi girişe kaydedilmez; kayıt target/ altındaki binary'yi gösterirdi.
    if cfg!(debug_assertions) || marker.exists() {
        return;
    }
    match app.autolaunch().enable() {
        Ok(()) => {
            let _ = std::fs::create_dir_all(config_dir);
            let _ = std::fs::write(marker, b"");
        }
        Err(error) => diagnostics::write(&format!("Girişte başlatma açılamadı: {error}")),
    }
}
