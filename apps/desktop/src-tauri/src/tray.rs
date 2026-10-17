//! Menü çubuğu (macOS) / sistem tepsisi (Windows) simgesi ve menüsü.
//!
//! Menü: durum satırı · ✓ Pano paylaşımı · (güncelleme varsa) Güncelle ·
//! OpenContinuity'yi aç · Ayarlar… · Çık. Güncelleme öğesi yalnız gerektiğinde
//! görünsün diye menü o durum değişince yeniden kurulur.

use std::sync::Mutex;

use oc_engine::Command;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::updater::{self, UpdateStatus};
use crate::{AppState, View, show_window};

const TRAY_ID: &str = "main";

struct TrayItems {
    status: MenuItem<Wry>,
    sharing: CheckMenuItem<Wry>,
    /// Menüde güncelleme öğesi varsa metni.
    update_label: Option<String>,
}

#[derive(Default)]
struct TrayState(Mutex<Option<TrayItems>>);

#[derive(Clone, Copy)]
enum Look {
    Connected,
    Waiting,
    Paused,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    app.manage(TrayState::default());
    let menu = build_menu(app, None)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(Look::Waiting))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("OpenContinuity")
        .menu(&menu)
        // Windows'ta sol tık pencereyi açar, sağ tık menüyü; macOS'ta tık menüyü açar.
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(on_menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event
                && !cfg!(target_os = "macos")
            {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn build_menu(app: &AppHandle, update_label: Option<String>) -> tauri::Result<Menu<Wry>> {
    let status = MenuItem::with_id(app, "status", "Başlatılıyor…", false, None::<&str>)?;
    let sharing = CheckMenuItem::with_id(app, "sharing", "Pano paylaşımı", true, true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "OpenContinuity'yi aç", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ayarlar…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çık", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&status, &PredefinedMenuItem::separator(app)?, &sharing])?;
    if let Some(label) = &update_label {
        menu.append(&MenuItem::with_id(app, "update", label, true, None::<&str>)?)?;
    }
    menu.append_items(&[&PredefinedMenuItem::separator(app)?, &open, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;
    *app.state::<TrayState>().0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
        Some(TrayItems { status, sharing, update_label });
    Ok(menu)
}

pub fn refresh(app: &AppHandle, view: &View) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let wanted_update = update_label(&view.update);
    let current_update = app
        .state::<TrayState>()
        .0
        .lock()
        .ok()
        .and_then(|items| items.as_ref().map(|items| items.update_label.clone()));
    if current_update != Some(wanted_update.clone())
        && let Ok(menu) = build_menu(app, wanted_update)
    {
        let _ = tray.set_menu(Some(menu));
    }
    if let Ok(items) = app.state::<TrayState>().0.lock()
        && let Some(items) = items.as_ref()
    {
        let _ = items.status.set_text(status_text(view));
        let _ = items.sharing.set_checked(!view.snapshot.paused);
        let _ = items.sharing.set_enabled(view.snapshot.peer.is_some());
    }
    set_look(&tray, look(view));
    let _ = tray.set_tooltip(Some(format!("OpenContinuity — {}", status_text(view))));
}

fn update_label(status: &UpdateStatus) -> Option<String> {
    match status {
        UpdateStatus::Available { version, .. } => Some(format!("v{version} güncellemesini kur")),
        UpdateStatus::Downloading { percent: Some(percent), .. } => Some(format!("Güncelleme indiriliyor… %{percent}")),
        UpdateStatus::Downloading { .. } | UpdateStatus::Installing { .. } => Some("Güncelleniyor…".into()),
        _ => None,
    }
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let state = app.state::<AppState>();
    match event.id().as_ref() {
        "sharing" => {
            let paused = state.view.lock().map(|view| view.snapshot.paused).unwrap_or(false);
            state.engine().send(if paused { Command::Resume } else { Command::Pause });
        }
        "update" => {
            if matches!(updater::current(app), UpdateStatus::Available { .. }) {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = updater::install(&app).await;
                });
            }
        }
        "open" => show_window(app),
        "settings" => {
            show_window(app);
            let _ = app.emit("open-settings", ());
        }
        "quit" => {
            state.engine().send(Command::Shutdown);
            app.exit(0);
        }
        _ => {}
    }
}

fn status_text(view: &View) -> String {
    let snapshot = &view.snapshot;
    if view.engine_error.is_some() {
        return "Çalışmıyor — ayrıntı için açın".into();
    }
    if snapshot.pairing.is_some() {
        return "Eşleştiriliyor…".into();
    }
    let Some(peer) = &snapshot.peer else {
        return "Eşleşmiş cihaz yok".into();
    };
    if snapshot.paused {
        format!("Duraklatıldı · {}", peer.name)
    } else if snapshot.connected_ip.is_some() {
        format!("Bağlı · {}", peer.name)
    } else {
        format!("Bağlantı bekleniyor · {}", peer.name)
    }
}

fn look(view: &View) -> Look {
    match (&view.snapshot.connected_ip, view.snapshot.paused) {
        (_, true) => Look::Paused,
        (Some(_), false) if view.engine_error.is_none() => Look::Connected,
        _ => Look::Waiting,
    }
}

fn set_look(tray: &TrayIcon, look: Look) {
    let _ = tray.set_icon(Some(icon(look)));
    // macOS'ta simge değişince şablon bayrağı yeniden verilmeli (koyu/açık menü çubuğu).
    let _ = tray.set_icon_as_template(cfg!(target_os = "macos"));
}

fn icon(look: Look) -> Image<'static> {
    #[cfg(target_os = "macos")]
    let bytes: &[u8] = match look {
        Look::Connected => include_bytes!("../icons/tray/mac-connected.png"),
        Look::Waiting => include_bytes!("../icons/tray/mac-waiting.png"),
        Look::Paused => include_bytes!("../icons/tray/mac-paused.png"),
    };
    #[cfg(not(target_os = "macos"))]
    let bytes: &[u8] = match look {
        Look::Connected => include_bytes!("../icons/tray/win-connected.png"),
        Look::Waiting => include_bytes!("../icons/tray/win-waiting.png"),
        Look::Paused => include_bytes!("../icons/tray/win-paused.png"),
    };
    Image::from_bytes(bytes).expect("gömülü tepsi simgesi bozuk")
}
