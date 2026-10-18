// Windows sürüm derlemesinde konsol penceresi açılmasın.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod commands;
mod diagnostics;
mod tray;
mod updater;

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use oc_engine::{ClipboardSource, ContentKind, Engine, EngineConfig, Event, Handle, Snapshot};
use oc_pairing::PeerStore;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug, Clone, Serialize)]
pub struct Transfer {
    /// "sent" | "received" | "too_large" (sınırı aştığı için gönderilmedi)
    direction: &'static str,
    content: ContentKind,
    bytes: u64,
    /// Dosyada kopyalanan üst düzey öğe sayısı.
    items: usize,
    at_ms: u64,
    /// Gelen dosyaların klasörü; arayüz "Göster" der. Yol arayüzden geri alınmaz.
    #[serde(skip)]
    saved_to: Option<std::path::PathBuf>,
    can_reveal: bool,
}

impl Transfer {
    fn now(direction: &'static str, content: ContentKind, bytes: u64, items: usize) -> Self {
        let at_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0);
        Self { direction, content, bytes, items, at_ms, saved_to: None, can_reveal: false }
    }

    fn saved_to(mut self, dir: Option<std::path::PathBuf>) -> Self {
        self.can_reveal = dir.is_some();
        self.saved_to = dir;
        self
    }
}

/// Arayüzde gösterilen son aktarım sayısı.
const RECENT_TRANSFERS: usize = 5;

/// Arayüzün çizdiği her şey; her değişimde `view` olayıyla tamamı gönderilir.
#[derive(Debug, Clone, Default, Serialize)]
pub struct View {
    snapshot: Snapshot,
    /// En yenisi başta.
    recent_transfers: Vec<Transfer>,
    /// Süren görsel aktarımı; bitince, atlanınca ya da bağlantı kopunca temizlenir.
    in_flight: Option<Transfer>,
    pairing_error: Option<String>,
    /// Motor durduysa (ör. port kullanımda) nedeni.
    engine_error: Option<String>,
    update: updater::UpdateStatus,
}

impl View {
    pub fn saved_folder(&self, at_ms: u64) -> Option<std::path::PathBuf> {
        self.recent_transfers.iter().find(|transfer| transfer.at_ms == at_ms)?.saved_to.clone()
    }

    fn record_transfer(&mut self, transfer: Transfer) {
        self.in_flight = None;
        self.recent_transfers.insert(0, transfer);
        self.recent_transfers.truncate(RECENT_TRANSFERS);
    }
}

pub struct AppState {
    /// Motor yeniden başlatılınca değişir.
    handle: Mutex<Handle>,
    view: Mutex<View>,
}

impl AppState {
    pub fn engine(&self) -> Handle {
        self.handle.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }
}

fn main() {
    let mut builder = tauri::Builder::default();
    // OC_CONFIG_DIR ile aynı makinede ikinci örnek açılabilsin (eşleştirme denemesi).
    if std::env::var_os("OC_CONFIG_DIR").is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)));
    }
    let app = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::restart_engine,
            commands::check_update,
            commands::install_update,
            commands::open_local_network_settings,
            commands::open_project_page,
            commands::reveal_transfer,
            commands::open_files_dir,
            diagnostics::read_log,
            diagnostics::save_log_to_desktop,
            diagnostics::reveal_log,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let config = EngineConfig::system()?;
            let unpaired = PeerStore::new(&config.config_dir).load()?.is_none();
            diagnostics::open(&config.config_dir);
            autostart::sync(app.handle(), &config.config_dir);
            let (engine, handle, events) = Engine::new(config, ClipboardSource::System);
            app.manage(AppState { handle: Mutex::new(handle), view: Mutex::new(View::default()) });
            app.manage(updater::PendingUpdate::default());
            tray::create(app.handle())?;
            spawn_engine(app.handle().clone(), engine, events);
            updater::spawn_periodic_check(app.handle().clone());
            // Elle açılışta pencere görünür; girişte otomatik açılışta eş varsa yalnız simge.
            let autostarted = std::env::args().any(|arg| arg == autostart::AUTOSTART_ARG);
            if unpaired || !autostarted {
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
    app.run(|app, event| {
        // Yalnız macOS kolunda kullanılır.
        #[cfg(not(target_os = "macos"))]
        let _ = app;
        match event {
            RunEvent::ExitRequested { api, code: None, .. } => api.prevent_exit(),
            // macOS'ta çalışan uygulamanın simgesine tıklanınca ikinci süreç başlamaz,
            // yalnız bu olay gelir; pencere açılmazsa uygulama "açılmıyor" görünür.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => show_window(app),
            _ => {}
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

pub fn spawn_engine(app: AppHandle, engine: Engine, mut events: UnboundedReceiver<Event>) {
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
        Event::Sending { content, bytes, items } => {
            update_view(app, |view| view.in_flight = Some(Transfer::now("sent", content, bytes, items)))
        }
        Event::Receiving { content, bytes, items } => {
            update_view(app, |view| view.in_flight = Some(Transfer::now("received", content, bytes, items)))
        }
        Event::Sent { content, bytes, items, .. } => {
            update_view(app, |view| view.record_transfer(Transfer::now("sent", content, bytes, items)))
        }
        Event::Received { content, bytes, items, saved_to, .. } => {
            update_view(app, |view| view.record_transfer(Transfer::now("received", content, bytes, items).saved_to(saved_to)))
        }
        Event::TooLarge { content, bytes, .. } => {
            update_view(app, |view| view.record_transfer(Transfer::now("too_large", content, bytes, 1)))
        }
        Event::Skipped { .. } | Event::Disconnected { .. } => update_view(app, |view| view.in_flight = None),
        Event::PairingFailed { reason } => update_view(app, |view| view.pairing_error = Some(reason)),
        Event::Paired { .. } => update_view(app, |view| view.pairing_error = None),
        Event::Diagnostic { .. } => {}
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
