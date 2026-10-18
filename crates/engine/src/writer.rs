//! Bağlantının yazma görevi. Kontrol mesajları hemen yazılır; görsel ve dosya
//! aktarımı parça parça ilerler ve her parçadan önce yeni iş var mı bakılır.
//! Yeni pano olayı gelirse süren aktarım `TransferCancel` ile bırakılır.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, anyhow};
use oc_protocol::{Chunk, ClipEvent, ContentKind, IMAGE_CHUNK_BYTES, Message, Payload, split, write_message};
use oc_transport::Stream;
use tokio::io::{AsyncReadExt, WriteHalf};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use crate::link::LinkId;

pub enum Job {
    Control { message: Message, ack: Option<oneshot::Sender<anyhow::Result<()>>> },
    Clip { event: ClipEvent, sources: Vec<PathBuf> },
}

#[derive(Debug)]
pub struct Completion {
    pub content: ContentKind,
    pub bytes: u64,
    pub items: usize,
    pub lamport: u64,
    pub outcome: Outcome,
}

#[derive(Debug)]
pub enum Outcome {
    Delivered,
    /// Yeni kopya geldi; bu aktarım yarıda bırakıldı.
    Superseded,
    /// Kaynak dosya okunamadı ya da aktarım sırasında değişti.
    SourceFailed(String),
    /// Yazılamadı; bağlantı kullanılamaz.
    LinkFailed(String),
}

struct Bulk {
    report: Report,
    frames: Frames,
}

#[derive(Clone, Copy)]
struct Report {
    content: ContentKind,
    bytes: u64,
    items: usize,
    lamport: u64,
}

enum Frames {
    Image(VecDeque<Message>),
    Files { start: Option<Message>, sources: VecDeque<(PathBuf, u64)>, file: Option<(tokio::fs::File, u64)> },
}

pub async fn write_loop(
    link_id: LinkId,
    mut writer: WriteHalf<Stream>,
    mut jobs: mpsc::UnboundedReceiver<Job>,
    completions: mpsc::UnboundedSender<(LinkId, Completion)>,
    send_timeout: Duration,
) {
    let write = async |writer: &mut WriteHalf<Stream>, message: &Message| {
        timeout(send_timeout, write_message(writer, message)).await.context("gönderim zaman aşımı")?
    };
    let finish = |report: Report, outcome: Outcome| {
        let completion =
            Completion { content: report.content, bytes: report.bytes, items: report.items, lamport: report.lamport, outcome };
        let _ = completions.send((link_id, completion));
    };
    let mut bulk: Option<Bulk> = None;
    loop {
        let job = if bulk.is_some() {
            match jobs.try_recv() {
                Ok(job) => Some(job),
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => return,
            }
        } else {
            match jobs.recv().await {
                Some(job) => Some(job),
                None => return,
            }
        };
        match job {
            Some(Job::Control { message, ack }) => {
                let result = write(&mut writer, &message).await;
                let failed = result.as_ref().err().map(|error| format!("{error:#}"));
                if let Some(ack) = ack {
                    let _ = ack.send(result);
                }
                if let Some(reason) = failed {
                    if let Some(current) = bulk.take() {
                        finish(current.report, Outcome::LinkFailed(reason));
                    }
                    return;
                }
            }
            Some(Job::Clip { event, sources }) => {
                if let Some(current) = bulk.take() {
                    if let Err(error) = write(&mut writer, &Message::TransferCancel).await {
                        finish(current.report, Outcome::LinkFailed(format!("{error:#}")));
                        return;
                    }
                    finish(current.report, Outcome::Superseded);
                }
                let report = report_of(&event);
                match event.payload {
                    Payload::Text(_) => {
                        let result = write(&mut writer, &Message::Clip(event)).await;
                        let failed = result.is_err();
                        finish(report, result.map_or_else(|error| Outcome::LinkFailed(format!("{error:#}")), |()| Outcome::Delivered));
                        if failed {
                            return;
                        }
                    }
                    Payload::Png(_) => bulk = Some(Bulk { report, frames: Frames::Image(split(Message::Clip(event)).into()) }),
                    Payload::Files(files) => {
                        let sizes = files.entries.iter().filter(|entry| !entry.is_dir).map(|entry| entry.size);
                        let sources = sources.into_iter().zip(sizes).collect();
                        let start = Message::FilesStart { event_id: event.event_id, lamport: event.lamport, files };
                        bulk = Some(Bulk { report, frames: Frames::Files { start: Some(start), sources, file: None } });
                    }
                }
            }
            None => {
                let current = bulk.as_mut().expect("aktarım sürüyor");
                match current.frames.next().await {
                    Ok(Some(frame)) => {
                        if let Err(error) = write(&mut writer, &frame).await {
                            finish(current.report, Outcome::LinkFailed(format!("{error:#}")));
                            return;
                        }
                    }
                    Ok(None) => finish(bulk.take().expect("aktarım sürüyor").report, Outcome::Delivered),
                    Err(error) => {
                        let report = bulk.take().expect("aktarım sürüyor").report;
                        if let Err(write_error) = write(&mut writer, &Message::TransferCancel).await {
                            finish(report, Outcome::LinkFailed(format!("{write_error:#}")));
                            return;
                        }
                        finish(report, Outcome::SourceFailed(format!("{error:#}")));
                    }
                }
            }
        }
    }
}

fn report_of(event: &ClipEvent) -> Report {
    let items = match &event.payload {
        Payload::Files(files) => files.top_level_count(),
        _ => 1,
    };
    Report { content: event.payload.kind(), bytes: event.payload.len(), items, lamport: event.lamport }
}

impl Frames {
    /// Sıradaki çerçeve; bitince `None`. Dosya bildirilen boyuttan kısa ya da
    /// uzun çıkarsa hata (karşı taraf yarım aktarımı atar).
    async fn next(&mut self) -> anyhow::Result<Option<Message>> {
        match self {
            Frames::Image(frames) => Ok(frames.pop_front()),
            Frames::Files { start, sources, file } => {
                if let Some(start) = start.take() {
                    return Ok(Some(start));
                }
                loop {
                    if let Some((handle, remaining)) = file.as_mut() {
                        let mut buffer = vec![0; IMAGE_CHUNK_BYTES.min(*remaining as usize)];
                        handle.read_exact(&mut buffer).await.context("dosya okunurken kısaldı")?;
                        *remaining -= buffer.len() as u64;
                        if *remaining == 0 {
                            let mut probe = [0u8; 1];
                            if handle.read(&mut probe).await? != 0 {
                                return Err(anyhow!("dosya gönderilirken büyüdü"));
                            }
                            *file = None;
                        }
                        return Ok(Some(Message::FileChunk { data: Chunk(buffer) }));
                    }
                    let Some((path, size)) = sources.pop_front() else { return Ok(None) };
                    if size > 0 {
                        let handle = tokio::fs::File::open(&path).await.with_context(|| format!("{} açılamadı", path.display()))?;
                        *file = Some((handle, size));
                    }
                }
            }
        }
    }
}
