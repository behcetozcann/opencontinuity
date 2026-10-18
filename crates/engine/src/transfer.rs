//! Senkron döngüsünün pano tarafı: yerel kopyayı gönderir, gelen olayı uygular,
//! aktarım sonuçlarını olaya çevirir. Bağlantı yönetimi `sync_loop`'ta kalır.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use oc_core::SyncState;
use oc_protocol::{ClipEvent, ContentKind, DeviceId, Message, Payload};

use crate::events::Event;
use crate::files::{self, Collected};
use crate::link::{Completion, Link, Outcome, is_bulk};
use crate::{Clip, Context};

/// Bağlantı kopukken yapılan son kopya; bağlanınca gönderilir.
pub struct Pending {
    event: ClipEvent,
    sources: Vec<PathBuf>,
    // Duvar saati: macOS'ta `Instant` uykuda ilerlemez, uykudan önceki kopya taze görünürdü.
    copied_at: SystemTime,
}

/// Yerel kopyayı gönderir. Bağlantı yazma görevi bittiyse hata döner (bağlantı koptu).
pub async fn send_local(
    context: &mut Context,
    state: &mut SyncState,
    link: &mut Option<Link>,
    pending: &mut Option<Pending>,
    copy: Clip,
) -> anyhow::Result<()> {
    // Duraklatılmışken saat ilerletilmez; yoksa eşin sonraki kopyaları eski görünürdü.
    if context.events.snapshot().paused {
        return Ok(());
    }
    let (payload, sources) = match copy {
        Clip::Text(text) => (Payload::Text(text), Vec::new()),
        Clip::Png(png) => (Payload::Png(png), Vec::new()),
        Clip::Files(roots) => {
            // Büyük klasörü gezmek zaman alabilir; çalışma ortamını tıkamasın.
            match tokio::task::spawn_blocking(move || files::collect(&roots)).await? {
                Ok(Collected::Ready(outgoing)) => (Payload::Files(outgoing.list), outgoing.sources),
                Ok(Collected::TooLarge { bytes }) => {
                    let limit = oc_protocol::MAX_FILES_BYTES;
                    context.events.emit(Event::TooLarge { content: ContentKind::Files, bytes, limit });
                    return Ok(());
                }
                Err(error) => {
                    context.events.emit(Event::Skipped { reason: format!("Dosyalar okunamadı: {error:#}") });
                    return Ok(());
                }
            }
        }
    };
    let (content, bytes, limit) = (payload.kind(), payload.len(), payload.max_bytes());
    if bytes > limit {
        context.events.emit(Event::TooLarge { content, bytes, limit });
        return Ok(());
    }
    let Some(event) = state.on_local(payload) else { return Ok(()) };
    let Some(active) = link.as_mut() else {
        let minutes = context.config.timing.offline_copy_max_age.as_secs() / 60;
        let reason = format!("Eş bağlı değil; {minutes} dk içinde bağlanırsa gönderilecek.");
        context.events.emit(Event::Skipped { reason });
        *pending = Some(Pending { event, sources, copied_at: SystemTime::now() });
        return Ok(());
    };
    *pending = None;
    send_event(context, active, event, sources)
}

/// Kopukken yapılan kopyayı yeni kurulan bağlantıdan gönderir; eskiyse bırakır.
/// Olay kopyalandığı andaki saatle gider: eş o arada kendisi kopyaladıysa
/// iki taraf da Lamport kuralıyla aynı sonuca varır.
pub fn send_pending(context: &mut Context, link: &mut Link, pending: &mut Option<Pending>) -> anyhow::Result<()> {
    let Some(Pending { event, sources, copied_at }) = pending.take() else { return Ok(()) };
    if context.events.snapshot().paused {
        return Ok(());
    }
    // Saat geri alındıysa süre bilinemez; kopya eski sayılır.
    let age = copied_at.elapsed().unwrap_or(Duration::MAX);
    if age > context.config.timing.offline_copy_max_age {
        let reason = format!("Bağlantı kopukken yapılan kopya {} sn önceydi; gönderilmedi.", age.as_secs());
        context.events.emit(Event::Skipped { reason });
        return Ok(());
    }
    context.events.diagnostic(format!("Bağlantı kopukken yapılan kopya ({} sn önce) gönderiliyor.", age.as_secs()));
    send_event(context, link, event, sources)
}

fn send_event(context: &mut Context, link: &mut Link, event: ClipEvent, sources: Vec<PathBuf>) -> anyhow::Result<()> {
    let (content, bytes, items) = (event.payload.kind(), event.payload.len(), items_of(&event.payload));
    link.send_clip(event, sources)?;
    if is_bulk(content) {
        context.events.emit(Event::Sending { content, bytes, items });
    }
    Ok(())
}

/// Bağlantı kullanılamaz hale geldiyse nedeni döner.
pub fn on_completion(context: &mut Context, completion: Completion) -> Option<String> {
    let Completion { content, bytes, items, lamport, outcome } = completion;
    match outcome {
        Outcome::Delivered => context.events.emit(Event::Sent { content, bytes, items, lamport }),
        Outcome::Superseded => {
            context.events.emit(Event::Skipped { reason: "Yeni kopya geldi; önceki aktarım bırakıldı.".into() });
        }
        Outcome::SourceFailed(reason) => context.events.emit(Event::Skipped { reason: format!("Gönderilemedi: {reason}") }),
        Outcome::LinkFailed(reason) => return Some(reason),
    }
    None
}

pub fn apply_remote(context: &mut Context, state: &mut SyncState, sender: &DeviceId, event: ClipEvent) -> anyhow::Result<()> {
    let (content, bytes, items) = (event.payload.kind(), event.payload.len(), items_of(&event.payload));
    let files_dir = context.config.files_dir.clone();
    if context.events.snapshot().paused {
        // Uygulanmasa da saat ilerler; yoksa devamdaki ilk yerel kopya eşin
        // son kopyasıyla aynı lamport'u alıp reddedilebilirdi.
        state.observe_peer_clock(event.lamport);
        files::discard(&files_dir, &event.event_id);
        context.events.emit(Event::Skipped { reason: format!("Duraklatıldı; gelen kopya ({bytes} bayt) uygulanmadı.") });
        return Ok(());
    }
    if !state.on_remote(sender, &event) {
        files::discard(&files_dir, &event.event_id);
        let reason = format!("Alındı ama uygulanmadı ({bytes} bayt): yerel kopya daha yeni ya da tekrar.");
        context.events.emit(Event::Skipped { reason });
        return Ok(());
    }
    let (clip, saved_to) = match event.payload {
        Payload::Text(text) => (Clip::Text(text), None),
        Payload::Png(png) => (Clip::Png(png), None),
        Payload::Files(list) => match files::accept(&files_dir, &event.event_id, &list) {
            Ok((saved, tops)) => (Clip::Files(tops), Some(saved)),
            Err(error) => {
                files::discard(&files_dir, &event.event_id);
                context.events.emit(Event::Skipped { reason: format!("Gelen dosyalar kaydedilemedi: {error:#}") });
                return Ok(());
            }
        },
    };
    context.events.emit(Event::Received { content, bytes, items, lamport: event.lamport, saved_to });
    context.clipboard.writes.send(clip).map_err(|_| anyhow!("pano iş parçacığı durdu"))
}

/// Okuma görevinin aktarım bildirimleri (başladı/bırakıldı).
pub fn on_notice(context: &mut Context, message: &Message) {
    let paused = context.events.snapshot().paused;
    match message {
        Message::ImageStart { total_bytes, .. } if !paused => {
            context.events.emit(Event::Receiving { content: ContentKind::Image, bytes: u64::from(*total_bytes), items: 1 });
        }
        Message::FilesStart { files, .. } if !paused => {
            let (bytes, items) = (files.total_bytes(), files.top_level_count());
            context.events.emit(Event::Receiving { content: ContentKind::Files, bytes, items });
        }
        Message::TransferCancel => context.events.emit(Event::Skipped { reason: "Gönderen aktarımı bıraktı.".into() }),
        _ => {}
    }
}

fn items_of(payload: &Payload) -> usize {
    match payload {
        Payload::Files(list) => list.top_level_count(),
        _ => 1,
    }
}
