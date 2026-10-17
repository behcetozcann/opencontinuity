//! Pano nesneleri iş parçacıkları arasında taşınamadığı (macOS) için pano
//! ayrı bir iş parçacığında yaşar; async tarafla kanallarla konuşur.

use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use oc_clipboard::Content;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::ClipboardPort;
use crate::events::Event;

const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Teşhis satırları (sayaç, sahip süreç, hash öneki) olay kanalına yazılır.
pub fn spawn(events: mpsc::UnboundedSender<Event>) -> anyhow::Result<ClipboardPort> {
    let log = move |message: String| {
        let _ = events.send(Event::Diagnostic { message });
    };
    let (local_sender, local_texts) = mpsc::channel(8);
    let (writes, write_receiver) = std_mpsc::channel::<String>();
    let (ready_sender, ready_receiver) = std_mpsc::channel();
    std::thread::Builder::new().name("pano".into()).spawn(move || {
        let clipboard = match oc_clipboard::system() {
            Ok(clipboard) => {
                let _ = ready_sender.send(Ok(()));
                clipboard
            }
            Err(error) => {
                let _ = ready_sender.send(Err(error));
                return;
            }
        };
        // Açılıştaki pano içeriği gönderilmez; yalnız bundan sonraki kopyalar.
        let mut last_sequence = clipboard.sequence();
        loop {
            match write_receiver.recv_timeout(POLL_INTERVAL) {
                Ok(text) => {
                    let before = clipboard.sequence();
                    match clipboard.write_text(&text) {
                        Ok(()) => log(format!(
                            "[pano] uzak metin yazıldı ({}), sayaç {before}→{}",
                            fingerprint(&text),
                            clipboard.sequence()
                        )),
                        Err(error) => log(format!("[pano] yazılamadı: {error}")),
                    }
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => {}
                Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
            }
            let sequence = clipboard.sequence();
            if sequence == last_sequence {
                continue;
            }
            let previous_sequence = std::mem::replace(&mut last_sequence, sequence);
            let content = oc_clipboard::read_after_change(clipboard.as_ref());
            let summary = match &content {
                Ok(Content::Text(text)) => fingerprint(text),
                Ok(Content::Concealed) => "gizli/geçici, atlandı".to_owned(),
                Ok(Content::NoText) => "düz metin yok".to_owned(),
                Err(error) => format!("okunamadı: {error}"),
            };
            log(format!(
                "[pano] sayaç {previous_sequence}→{sequence}: {summary}; {}",
                clipboard.describe_source()
            ));
            if let Ok(Content::Text(text)) = content
                && local_sender.blocking_send(text).is_err()
            {
                return;
            }
        }
    })?;
    ready_receiver.recv()??;
    Ok(ClipboardPort { local_texts, writes })
}

/// İçeriği değil yalnız uzunluk ve hash önekini gösterir.
fn fingerprint(text: &str) -> String {
    format!("{} bayt, hash {}", text.len(), &hex::encode(Sha256::digest(text.as_bytes()))[..8])
}
