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
    oc_clipboard::system()?.write_text(text)?;
    println!("Panoya yazıldı: {}", describe(&Content::Text(text.to_owned())));
    Ok(())
}

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
        Content::Concealed => "gizli/geçici işaretli (gönderilmez)".to_owned(),
        Content::NoText => "düz metin yok".to_owned(),
    }
}
