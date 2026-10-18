//! Pano nesneleri iş parçacıkları arasında taşınamadığı (macOS) için pano
//! ayrı bir iş parçacığında yaşar; async tarafla kanallarla konuşur.

use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use oc_clipboard::{Clipboard, Content};
use oc_protocol::Payload;
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
    let (local_sender, local_copies) = mpsc::channel(8);
    let (writes, write_receiver) = std_mpsc::channel::<Payload>();
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
                Ok(payload) => {
                    let before = clipboard.sequence();
                    match write(clipboard.as_ref(), &payload) {
                        Ok(()) => log(format!(
                            "[pano] uzak içerik yazıldı ({}), sayaç {before}→{}",
                            fingerprint(&payload),
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
            let payload = match oc_clipboard::read_after_change(clipboard.as_ref()) {
                Ok(Content::Text(text)) => Ok(Payload::Text(text)),
                Ok(Content::Image(png)) => Ok(Payload::Png(png)),
                Ok(Content::Concealed) => Err("gizli/geçici, atlandı".to_owned()),
                Ok(Content::Unsupported) => Err("desteklenen içerik yok".to_owned()),
                Err(error) => Err(format!("okunamadı: {error}")),
            };
            let summary = payload.as_ref().map_or_else(Clone::clone, fingerprint);
            log(format!(
                "[pano] sayaç {previous_sequence}→{sequence}: {summary}; {}",
                clipboard.describe_source()
            ));
            if let Ok(payload) = payload
                && local_sender.blocking_send(payload).is_err()
            {
                return;
            }
        }
    })?;
    ready_receiver.recv()??;
    Ok(ClipboardPort { local_copies, writes })
}

fn write(clipboard: &dyn Clipboard, payload: &Payload) -> anyhow::Result<()> {
    match payload {
        Payload::Text(text) => clipboard.write_text(text),
        Payload::Png(png) => clipboard.write_png(png),
    }
}

/// İçeriği değil yalnız tür, uzunluk ve hash önekini gösterir.
fn fingerprint(payload: &Payload) -> String {
    let kind = match payload {
        Payload::Text(_) => "metin",
        Payload::Png(_) => "görsel",
    };
    format!("{kind}, {} bayt, hash {}", payload.len(), &hex::encode(Sha256::digest(payload.as_bytes()))[..8])
}
