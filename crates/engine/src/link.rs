//! Kurulmuş bir oturumun okuma/yazma uçları. Okuma ayrı görevde yapılır:
//! `read_message` `select!` içinde iptal güvenli değildir.

use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use oc_protocol::{Message, read_message, write_message};
use oc_transport::{Session, Stream};
use tokio::io::WriteHalf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;

pub struct Link {
    writer: WriteHalf<Stream>,
    incoming: mpsc::Receiver<anyhow::Result<Message>>,
    pub last_seen: Instant,
    reader: JoinHandle<()>,
    send_timeout: Duration,
}

impl Link {
    pub fn start(session: Session, send_timeout: Duration) -> Self {
        let (mut reader_half, writer) = session.split();
        let (sender, incoming) = mpsc::channel(16);
        let reader = tokio::spawn(async move {
            loop {
                let message = read_message(&mut reader_half).await;
                let failed = message.is_err();
                if sender.send(message).await.is_err() || failed {
                    return;
                }
            }
        });
        Self { writer, incoming, last_seen: Instant::now(), reader, send_timeout }
    }

    pub async fn send(&mut self, message: &Message) -> anyhow::Result<()> {
        timeout(self.send_timeout, write_message(&mut self.writer, message)).await.context("gönderim zaman aşımı")?
    }

    pub async fn recv(&mut self) -> anyhow::Result<Message> {
        let message = self.incoming.recv().await.unwrap_or_else(|| Err(anyhow!("okuma görevi bitti")));
        if message.is_ok() {
            self.last_seen = Instant::now();
        }
        message
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
