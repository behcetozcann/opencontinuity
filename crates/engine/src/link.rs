//! Kurulmuş bir oturumun okuma/yazma uçları. Okuma ayrı görevde yapılır:
//! `read_message` `select!` içinde iptal güvenli değildir. Görseller burada
//! parçalanır ve birleştirilir; döngü bütün `Clip` olaylarını ve görselin
//! başladığını bildiren `ImageStart`'ı görür, parçaları görmez.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use oc_protocol::{Assembler, Message, read_message, split, write_message};
use oc_transport::{Session, Stream};
use tokio::io::WriteHalf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;

pub struct Link {
    writer: WriteHalf<Stream>,
    incoming: mpsc::Receiver<anyhow::Result<Message>>,
    /// Her çerçevede güncellenir; uzun görsel aktarımı sürerken eş canlı sayılır.
    last_seen: Arc<Mutex<Instant>>,
    reader: JoinHandle<()>,
    send_timeout: Duration,
}

impl Link {
    pub fn start(session: Session, send_timeout: Duration) -> Self {
        let (mut reader_half, writer) = session.split();
        let (sender, incoming) = mpsc::channel(16);
        let last_seen = Arc::new(Mutex::new(Instant::now()));
        let seen = last_seen.clone();
        // Okuma döngüden bağımsız sürer: iki uç aynı anda büyük görsel gönderse
        // de soketler boşaltılır, yazımlar birbirini kilitlemez.
        let reader = tokio::spawn(async move {
            let mut assembler = Assembler::default();
            loop {
                let message = match read_message(&mut reader_half).await {
                    Ok(message) => {
                        *seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Instant::now();
                        // Başlangıç ayrıca iletilir: arayüz "alınıyor" gösterebilsin.
                        if matches!(message, Message::ImageStart { .. }) && sender.send(Ok(message.clone())).await.is_err() {
                            return;
                        }
                        match assembler.push(message) {
                            Ok(None) => continue,
                            Ok(Some(message)) => Ok(message),
                            Err(error) => Err(error),
                        }
                    }
                    Err(error) => Err(error),
                };
                let failed = message.is_err();
                if sender.send(message).await.is_err() || failed {
                    return;
                }
            }
        });
        Self { writer, incoming, last_seen, reader, send_timeout }
    }

    /// Görsel olayları parça parça gönderilir; süre sınırı her parça için ayrıdır.
    pub async fn send(&mut self, message: Message) -> anyhow::Result<()> {
        for part in split(message) {
            timeout(self.send_timeout, write_message(&mut self.writer, &part)).await.context("gönderim zaman aşımı")??;
        }
        Ok(())
    }

    pub async fn recv(&mut self) -> anyhow::Result<Message> {
        self.incoming.recv().await.unwrap_or_else(|| Err(anyhow!("okuma görevi bitti")))
    }

    pub fn last_seen(&self) -> Instant {
        *self.last_seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.reader.abort();
    }
}

/// Bağlantı yoksa hiç dönmez.
pub async fn next_incoming(link: &mut Option<Link>) -> anyhow::Result<Message> {
    match link.as_mut() {
        Some(link) => link.recv().await,
        None => std::future::pending().await,
    }
}
