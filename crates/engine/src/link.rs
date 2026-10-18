//! Kurulmuş bir oturumun okuma/yazma uçları; ikisi de ayrı görevde çalışır.
//! Okuma: `read_message` `select!` içinde iptal güvenli değildir; görseller
//! burada birleştirilir, dosyalar diske yazılır, döngü bütün `Clip` olaylarını
//! ve aktarım başlangıç/iptal bildirimlerini görür.
//! Yazma: büyük aktarım parça parça akarken döngü beklemez; ping/pong araya
//! girer, yeni pano olayı süren aktarımı bırakır (en son kopya geçerlidir).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use oc_protocol::{Assembler, ClipEvent, ContentKind, Message, Payload, read_message};
use oc_transport::{Session, Stream};
use tokio::io::ReadHalf;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::files::FileSink;
use crate::writer::{Job, write_loop};

pub use crate::writer::{Completion, Outcome};

/// Bağlantının kimliği; aynı eşle yeniden kurulan bağlantının eski mesajları ayırt edilir.
pub type LinkId = u64;

/// Bütün bağlantıların okuma ve yazma sonuçları bu ortak kanallara bağlantı kimliğiyle düşer.
#[derive(Clone)]
pub struct Outlets {
    pub incoming: mpsc::Sender<(LinkId, anyhow::Result<Message>)>,
    pub completions: mpsc::UnboundedSender<(LinkId, Completion)>,
}

pub struct Inlets {
    pub incoming: mpsc::Receiver<(LinkId, anyhow::Result<Message>)>,
    pub completions: mpsc::UnboundedReceiver<(LinkId, Completion)>,
}

pub fn channels() -> (Outlets, Inlets) {
    let (incoming_sender, incoming) = mpsc::channel(16);
    let (completion_sender, completions) = mpsc::unbounded_channel();
    (Outlets { incoming: incoming_sender, completions: completion_sender }, Inlets { incoming, completions })
}

pub struct Link {
    pub id: LinkId,
    jobs: mpsc::UnboundedSender<Job>,
    /// Her çerçevede güncellenir; uzun aktarım sürerken eş canlı sayılır.
    last_seen: Arc<Mutex<Instant>>,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

impl Link {
    /// `files_dir`: gelen dosyaların yazılacağı kök.
    pub fn start(id: LinkId, session: Session, send_timeout: Duration, files_dir: PathBuf, outlets: &Outlets) -> Self {
        let (reader_half, writer_half) = session.split();
        let last_seen = Arc::new(Mutex::new(Instant::now()));
        let sender = Tagged { id, sender: outlets.incoming.clone() };
        let reader = tokio::spawn(read_loop(reader_half, sender, last_seen.clone(), FileSink::new(files_dir)));
        let (jobs, job_receiver) = mpsc::unbounded_channel();
        let writer = tokio::spawn(write_loop(id, writer_half, job_receiver, outlets.completions.clone(), send_timeout));
        Self { id, jobs, last_seen, reader, writer }
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

/// Okuma görevinin döngüye giden ucu.
struct Tagged {
    id: LinkId,
    sender: mpsc::Sender<(LinkId, anyhow::Result<Message>)>,
}

impl Tagged {
    async fn send(&self, message: anyhow::Result<Message>) -> anyhow::Result<()> {
        self.sender.send((self.id, message)).await.map_err(|_| anyhow!("döngü kapandı"))
    }
}

async fn read_loop(
    mut reader: ReadHalf<Stream>,
    sender: Tagged,
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
    sender: &Tagged,
) -> anyhow::Result<Option<Message>> {
    match message {
        Message::FilesStart { event_id, lamport, files } => {
            // Başlangıç ayrıca iletilir: arayüz "alınıyor" gösterebilsin.
            let notice = Message::FilesStart { event_id, lamport, files: files.clone() };
            sender.send(Ok(notice)).await?;
            Ok(sink.start(event_id, lamport, files).await?.map(completed_files))
        }
        Message::FileChunk { data } => Ok(sink.chunk(&data.0).await?.map(completed_files)),
        Message::TransferCancel => {
            assembler.cancel();
            sink.cancel().await;
            Ok(Some(Message::TransferCancel))
        }
        Message::ImageStart { .. } => {
            sender.send(Ok(message.clone())).await?;
            assembler.push(message)
        }
        other => assembler.push(other),
    }
}

fn completed_files(completed: crate::files::Completed) -> Message {
    Message::Clip(ClipEvent { event_id: completed.event_id, lamport: completed.lamport, payload: Payload::Files(completed.list) })
}

/// Arayüze "gönderiliyor/alınıyor" gösterilecek kadar uzun süren içerik mi.
pub fn is_bulk(content: ContentKind) -> bool {
    content != ContentKind::Text
}
