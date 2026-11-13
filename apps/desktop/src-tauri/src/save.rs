//! "Kaydet…": geçici klasördeki gelen dosyaları kullanıcının seçtiği klasöre kopyalar.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::AppState;

/// Kaydedilen klasörün yolunu döner; kullanıcı vazgeçerse `None`.
/// Kaynak yol arayüzden değil uygulamanın kaydından alınır.
#[tauri::command]
pub async fn save_transfer(app: AppHandle, state: State<'_, AppState>, at_ms: u64) -> Result<Option<String>, String> {
    let folder = state.view.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).saved_folder(at_ms);
    let folder = folder.ok_or("bu aktarımın dosyaları yok")?;
    if !folder.exists() {
        return Err("dosyalar artık yok (yeni bir dosya kopyası geldi)".into());
    }
    let picked = app.dialog().file().set_title("Gelen dosyaları kaydet").blocking_pick_folder();
    let Some(target) = picked.and_then(|path| path.into_path().ok()) else { return Ok(None) };
    let copied = target.clone();
    tauri::async_runtime::spawn_blocking(move || copy_items(&folder, &copied))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| format!("kaydedilemedi: {error}"))?;
    Ok(Some(target.to_string_lossy().into_owned()))
}

/// `source` içindeki her öğeyi `target`'a kopyalar; aynı ad varsa "ad (2).uzantı" olur.
fn copy_items(source: &Path, target: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let destination = free_name(target, &entry.file_name().to_string_lossy());
        copy_tree(&entry.path(), &destination)?;
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    if source.is_dir() {
        std::fs::create_dir(destination)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(source, destination).map(|_| ())
    }
}

fn free_name(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, format!(".{extension}")),
        _ => (name, String::new()),
    };
    (2..).map(|number| dir.join(format!("{stem} ({number}){extension}"))).find(|path| !path.exists()).expect("sonsuz aralık")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_files_and_folders_without_overwriting() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("not.txt"), b"yeni").unwrap();
        std::fs::create_dir_all(source.path().join("Klasör/iç")).unwrap();
        std::fs::write(source.path().join("Klasör/iç/a.bin"), b"abc").unwrap();
        std::fs::write(target.path().join("not.txt"), b"eski").unwrap();

        copy_items(source.path(), target.path()).unwrap();

        assert_eq!(std::fs::read(target.path().join("not.txt")).unwrap(), b"eski");
        assert_eq!(std::fs::read(target.path().join("not (2).txt")).unwrap(), b"yeni");
        assert_eq!(std::fs::read(target.path().join("Klasör/iç/a.bin")).unwrap(), b"abc");
    }
}
