use std::thread::sleep;
use std::time::{Duration, Instant};

use oc_clipboard::Content;
use sha2::{Digest, Sha256};

const POLL_INTERVAL: Duration = Duration::from_millis(250);

pub fn watch() -> anyhow::Result<()> {
    let clipboard = oc_clipboard::system()?;
    let started = Instant::now();
    let mut last_sequence = clipboard.sequence();
    println!("Pano izleniyor (sayaç {last_sequence}). Çıkmak için Ctrl+C.");
    loop {
        sleep(POLL_INTERVAL);
        let sequence = clipboard.sequence();
        if sequence == last_sequence {
            continue;
        }
        last_sequence = sequence;
        let elapsed = started.elapsed().as_secs_f32();
        match oc_clipboard::read_after_change(clipboard.as_ref()) {
            Ok(content) => println!("[{elapsed:7.1} sn] sayaç {sequence}: {}", describe(&content)),
            Err(error) => println!("[{elapsed:7.1} sn] sayaç {sequence}: okuma hatası: {error}"),
        }
    }
}

pub fn set(text: &str) -> anyhow::Result<()> {
    let clipboard = oc_clipboard::system()?;
    clipboard.write_text(text)?;
    println!("Panoya yazıldı: {}", describe(&Content::Text(text.to_owned())));
    hold(clipboard.as_ref());
    Ok(())
}

pub fn set_image(path: &std::path::Path) -> anyhow::Result<()> {
    let png = std::fs::read(path)?;
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        anyhow::bail!("{} bir PNG dosyası değil", path.display());
    }
    let clipboard = oc_clipboard::system()?;
    clipboard.write_png(&png)?;
    println!("Panoya yazıldı: {}", describe(&Content::Image(png)));
    hold(clipboard.as_ref());
    Ok(())
}

pub fn set_files(paths: &[std::path::PathBuf]) -> anyhow::Result<()> {
    let paths: Vec<_> = paths.iter().map(std::fs::canonicalize).collect::<Result<_, _>>()?;
    let clipboard = oc_clipboard::system()?;
    clipboard.write_files(&paths)?;
    println!("Panoya yazıldı: {}", describe(&Content::Files(paths)));
    hold(clipboard.as_ref());
    Ok(())
}

/// X11'de içeriği yazan süreç tutar; süreç çıkınca pano boşalır. xclip gibi
/// başka bir kopya gelene kadar beklenir.
#[cfg(target_os = "linux")]
fn hold(clipboard: &dyn oc_clipboard::Clipboard) {
    // Kendi yazmamızın sayaç artışı geçsin.
    sleep(POLL_INTERVAL);
    let written = clipboard.sequence();
    println!("Pano bu süreçte tutuluyor; başka bir kopya gelince çıkılır.");
    while clipboard.sequence() == written {
        sleep(POLL_INTERVAL);
    }
}

#[cfg(not(target_os = "linux"))]
fn hold(_clipboard: &dyn oc_clipboard::Clipboard) {}

/// İçeriği asla göstermez: yalnız uzunluk ve hash öneki.
fn describe(content: &Content) -> String {
    match content {
        Content::Text(text) => {
            let digest = Sha256::digest(text.as_bytes());
            format!(
                "metin, {} bayt, {} karakter, hash {}",
                text.len(),
                text.chars().count(),
                &hex::encode(digest)[..8]
            )
        }
        Content::Image(png) => format!("görsel (PNG), {} bayt, hash {}", png.len(), &hex::encode(Sha256::digest(png))[..8]),
        Content::Files(paths) => format!("dosya, {} öğe", paths.len()),
        Content::Concealed => "gizli/geçici işaretli (gönderilmez)".to_owned(),
        Content::Unsupported => "desteklenen içerik yok".to_owned(),
    }
}
