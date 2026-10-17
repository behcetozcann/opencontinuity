//! Uygulama içi güncelleme: GitHub Releases'teki imzalı `latest.json` denetlenir,
//! kullanıcı onaylayınca paket indirilir, imzası doğrulanır, kurulur ve uygulama
//! yeniden başlar. İmzası tutmayan paket kurulmaz (tauri-plugin-updater).

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{AppState, diagnostics, update_view};

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(15);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate {
        checked_at_ms: u64,
    },
    Available {
        version: String,
        notes: Option<String>,
    },
    Downloading {
        version: String,
        percent: Option<u8>,
    },
    Installing {
        version: String,
    },
    Failed {
        message: String,
    },
}

/// Denetimde bulunan, kurulmayı bekleyen sürüm.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

pub fn spawn_periodic_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            check(&app).await;
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

pub async fn check(app: &AppHandle) {
    if matches!(current(app), UpdateStatus::Checking | UpdateStatus::Downloading { .. } | UpdateStatus::Installing { .. }) {
        return;
    }
    set(app, UpdateStatus::Checking);
    let result = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    match result {
        Ok(Some(update)) => {
            diagnostics::write(&format!("Güncelleme var: {} → {}", update.current_version, update.version));
            let status = UpdateStatus::Available { version: update.version.clone(), notes: update.body.clone() };
            *pending(app) = Some(update);
            set(app, status);
        }
        Ok(None) => set(app, UpdateStatus::UpToDate { checked_at_ms: now_ms() }),
        Err(error) => {
            diagnostics::write(&format!("Güncelleme denetlenemedi: {error}"));
            set(app, UpdateStatus::Failed { message: format!("Güncelleme denetlenemedi: {error}") });
        }
    }
}

pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = pending(app).clone().ok_or("kurulacak güncelleme yok")?;
    let version = update.version.clone();
    diagnostics::write(&format!("Güncelleme indiriliyor: {version}"));
    set(app, UpdateStatus::Downloading { version: version.clone(), percent: None });
    let mut downloaded: u64 = 0;
    let mut last_percent = None;
    let result = update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                // Her parçada değil, yüzde değişince bildirilir.
                let percent = total.filter(|total| *total > 0).map(|total| (downloaded * 100 / total).min(100) as u8);
                if percent != last_percent {
                    last_percent = percent;
                    set(app, UpdateStatus::Downloading { version: version.clone(), percent });
                }
            },
            || set(app, UpdateStatus::Installing { version: version.clone() }),
        )
        .await;
    match result {
        Ok(()) => {
            diagnostics::write(&format!("Güncelleme kuruldu ({}), yeniden başlatılıyor", update.version));
            app.restart();
        }
        Err(error) => {
            let message = format!("Güncelleme kurulamadı: {error}");
            diagnostics::write(&message);
            set(app, UpdateStatus::Failed { message: message.clone() });
            Err(message)
        }
    }
}

pub fn current(app: &AppHandle) -> UpdateStatus {
    app.state::<AppState>().view.lock().map(|view| view.update.clone()).unwrap_or_default()
}

fn set(app: &AppHandle, status: UpdateStatus) {
    update_view(app, |view| view.update = status);
}

fn pending(app: &AppHandle) -> std::sync::MutexGuard<'_, Option<Update>> {
    app.state::<PendingUpdate>().inner().0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}
