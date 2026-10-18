//! Kurulmuş bir oturumun okuma/yazma uçları; ikisi de ayrı görevde çalışır.
//! Okuma: `read_message` `select!` içinde iptal güvenli değildir; görseller
//! burada birleştirilir, dosyalar diske yazılır, döngü bütün `Clip` olaylarını
//! ve aktarım başlangıç/iptal bildirimlerini görür.
//! Yazma: büyük aktarım parça parça akarken döngü beklemez; ping/pong araya
//! girer, yeni pano olayı süren aktarımı bırakır (en son kopya geçerlidir).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use oc_protocol::{Assembler, ClipEvent, ContentKind, Message, Payload, read_message};
use oc_transport::{Session, Stream};
use tokio::io::ReadHalf;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::files::FileSink;
use crate::writer::{Job, write_loop};

pub use crate::writer::{Completion, Outcome};

pub struct Link {
    jobs: mpsc::UnboundedSender<Job>,
    incoming: mpsc::Receiver<anyhow::Result<Message>>,
    completions: mpsc::UnboundedReceiver<Completion>,
    /// Her çerçevede güncellenir; uzun aktarım sürerken eş canlı sayılır.
    last_seen: Arc<Mutex<Instant>>,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

impl Link {
    /// `files_dir`: gelen dosyaların yazılacağı kök.
    pub fn start(session: Session, send_timeout: Duration, files_dir: PathBuf) -> Self {
        let (reader_half, writer_half) = session.split();
        let (sender, incoming) = mpsc::channel(16);
        let last_seen = Arc::new(Mutex::new(Instant::now()));
        let reader = tokio::spawn(read_loop(reader_half, sender, last_seen.clone(), FileSink::new(files_dir)));
        let (jobs, job_receiver) = mpsc::unbounded_channel();
        let (completion_sender, completions) = mpsc::unbounded_channel();
        let writer = tokio::spawn(write_loop(writer_half, job_receiver, completion_sender, send_timeout));
        Self { jobs, incoming, completions, last_seen, reader, writer }
    }

    /// Yazılana kadar bekler (el sıkışma sonrası `Clock`, eşleştirme kararı).
    pub async fn send(&mut self, message: Message) -> anyhow::Result<()> {
        let (ack, done) = oneshot::channel();
        self.jobs.send(Job::Control { message, ack: Some(ack) }).map_err(|_| anyhow!("yazma görevi bitti"))?;
        done.await.map_err(|_| anyhow!("yazma görevi bitti"))?
    }

    /// Beklemeden sıraya koyar (ping/pong); hata ancak bağlantı bitmişse.
    pub fn post(&mut self, message: Message) -> anyhow::Result<()> {
        self.jobs.send(Job::Control { message, ack: None }).map_err(|_| anyhow!("yazma görevi bitti"))
    }

    /// Pano olayını gönderir; sonucu `next_completion` bildirir. Dosyalarda
    /// `sources` listedeki dosyaların yollarıdır.
    pub fn send_clip(&mut self, event: ClipEvent, sources: Vec<PathBuf>) -> anyhow::Result<()> {
        self.jobs.send(Job::Clip { event, sources }).map_err(|_| anyhow!("yazma görevi bitti"))
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
        self.writer.abort();
    }
}

async fn read_loop(
    mut reader: ReadHalf<Stream>,
    sender: mpsc::Sender<anyhow::Result<Message>>,
    last_seen: Arc<Mutex<Instant>>,
    mut sink: FileSink,
) {
    let mut assembler = Assembler::default();
    loop {
        let message = match read_message(&mut reader).await {
            Ok(message) => {
                *last_seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Instant::now();
                handle(message, &mut assembler, &mut sink, &sender).await
            }
            Err(error) => Err(error),
        };
        let message = match message {
            Ok(None) => continue,
            Ok(Some(message)) => Ok(message),
            Err(error) => {
                sink.cancel().await;
                Err(error)
            }
        };
        let failed = message.is_err();
        if sender.send(message).await.is_err() || failed {
            return;
        }
    }
}

/// Döngüye iletilecek mesajı döner; parçalar için `None`.
async fn handle(
    message: Message,
    assembler: &mut Assembler,
    sink: &mut FileSink,
    sender: &mpsc::Sender<anyhow::Result<Message>>,
) -> anyhow::Result<Option<Message>> {
    match message {
        Message::FilesStart { event_id, lamport, files } => {
            // Başlangıç ayrıca iletilir: arayüz "alınıyor" gösterebilsin.
            let notice = Message::FilesStart { event_id, lamport, files: files.clone() };
            sender.send(Ok(notice)).await.context("döngü kapandı")?;
            Ok(sink.start(event_id, lamport, files).await?.map(completed_files))
        }
        Message::FileChunk { data } => Ok(sink.chunk(&data.0).await?.map(completed_files)),
        Message::TransferCancel => {
            assembler.cancel();
            sink.cancel().await;
            Ok(Some(Message::TransferCancel))
        }
        Message::ImageStart { .. } => {
            sender.send(Ok(message.clone())).await.context("döngü kapandı")?;
            assembler.push(message)
        }
        other => assembler.push(other),
    }
}

fn completed_files(completed: crate::files::Completed) -> Message {
    Message::Clip(ClipEvent { event_id: completed.event_id, lamport: completed.lamport, payload: Payload::Files(completed.list) })
}

pub enum LinkEvent {
    Incoming(anyhow::Result<Message>),
    Completed(Completion),
}

/// Gelen mesaj ya da biten gönderim. Bağlantı yoksa hiç dönmez.
pub async fn next_event(link: &mut Option<Link>) -> LinkEvent {
    let Some(link) = link.as_mut() else { return std::future::pending().await };
    tokio::select! {
        message = link.incoming.recv() => LinkEvent::Incoming(message.unwrap_or_else(|| Err(anyhow!("okuma görevi bitti")))),
        // Yazma görevi bittiyse okuma da hata verecek; o tarafı bekle.
        Some(completion) = link.completions.recv() => LinkEvent::Completed(completion),
    }
}

/// Arayüze "gönderiliyor/alınıyor" gösterilecek kadar uzun süren içerik mi.
pub fn is_bulk(content: ContentKind) -> bool {
    content != ContentKind::Text
}
