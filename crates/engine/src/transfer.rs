//! Senkron döngüsünün pano tarafı: yerel kopyayı gönderir, gelen olayı uygular,
//! aktarım sonuçlarını olaya çevirir. Bağlantı yönetimi `sync_loop`'ta kalır.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use oc_core::SyncState;
use oc_pairing::Fingerprint;
use oc_protocol::{ClipEvent, ContentKind, DeviceId, Message, Payload};

use crate::events::Event;
use crate::files::{self, Collected};
use crate::group::Group;
use crate::link::{Completion, Link, Outcome, is_bulk};
use crate::{Clip, Context};

/// Bağlantı kopukken yapılan son kopya; bağlanınca gönderilir.
pub struct Pending {
    event: ClipEvent,
    sources: Vec<PathBuf>,
    // Duvar saati: macOS'ta `Instant` uykuda ilerlemez, uykudan önceki kopya taze görünürdü.
    copied_at: SystemTime,
}

/// Yerel kopyayı bağlı bütün eşlere gönderir; bağlı olmayanlar için bekletir.
/// Yazma görevi biten (bağlantısı kopmuş) eşlerin parmak izini ve hatayı döner.
pub async fn send_local(
    context: &mut Context,
    state: &mut SyncState,
    group: &mut Group,
    copy: Clip,
) -> anyhow::Result<Vec<(Fingerprint, String)>> {
    // Duraklatılmışken saat ilerletilmez; yoksa eşin sonraki kopyaları eski görünürdü.
    if context.events.snapshot().paused {
        return Ok(Vec::new());
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
                    return Ok(Vec::new());
                }
                Err(error) => {
                    context.events.emit(Event::Skipped { reason: format!("Dosyalar okunamadı: {error:#}") });
                    return Ok(Vec::new());
                }
            }
        }
    };
    let (content, bytes, limit) = (payload.kind(), payload.len(), payload.max_bytes());
    if bytes > limit {
        context.events.emit(Event::TooLarge { content, bytes, limit });
        return Ok(Vec::new());
    }
    let Some(event) = state.on_local(payload) else { return Ok(Vec::new()) };
    let mut failed = Vec::new();
    let mut sent_any = false;
    for member in &mut group.members {
        let Some(link) = member.link.as_mut() else {
            member.pending = Some(Pending { event: event.clone(), sources: sources.clone(), copied_at: SystemTime::now() });
            continue;
        };
        member.pending = None;
        match link.send_clip(event.clone(), sources.clone()) {
            Ok(()) => sent_any = true,
            Err(error) => failed.push((member.peer.fingerprint, error.to_string())),
        }
    }
    if sent_any {
        report_sending(context, &event);
    } else {
        let minutes = context.config.timing.offline_copy_max_age.as_secs() / 60;
        let reason = format!("Bağlı cihaz yok; {minutes} dk içinde bağlanırsa gönderilecek.");
        context.events.emit(Event::Skipped { reason });
    }
    Ok(failed)
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
    // "Gönderiliyor" bildirilmez: kopya başka eşlere gittiyse sonucu zaten bildirildi.
    link.send_clip(event, sources)

}

fn report_sending(context: &mut Context, event: &ClipEvent) {
    let (content, bytes, items) = (event.payload.kind(), event.payload.len(), items_of(&event.payload));
    if is_bulk(content) {
        context.events.emit(Event::Sending { content, bytes, items });
    }
}

/// Bağlantı kullanılamaz hale geldiyse nedeni döner. Aynı kopya birden çok
/// eşe gittiği için sonuç kopya başına bir kez bildirilir (`reported`: son
/// bildirilen Lamport).
pub fn on_completion(context: &mut Context, completion: Completion, reported: &mut Option<u64>) -> Option<String> {
    let Completion { content, bytes, items, lamport, outcome } = completion;
    let first_report = *reported != Some(lamport);
    match outcome {
        Outcome::LinkFailed(reason) => return Some(reason),
        _ if !first_report => {}
        Outcome::Delivered => context.events.emit(Event::Sent { content, bytes, items, lamport }),
        Outcome::Superseded => {
            context.events.emit(Event::Skipped { reason: "Yeni kopya geldi; önceki aktarım bırakıldı.".into() });
        }
        Outcome::SourceFailed(reason) => context.events.emit(Event::Skipped { reason: format!("Gönderilemedi: {reason}") }),
    }
    *reported = Some(lamport);
    None
}

/// `sender`: olayı gönderen eşin kimliği ve adı.
pub fn apply_remote(context: &mut Context, state: &mut SyncState, sender: (&DeviceId, &str), event: ClipEvent) -> anyhow::Result<()> {
    let (sender, from) = (sender.0, sender.1.to_owned());
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
    context.events.emit(Event::Received { content, bytes, items, lamport: event.lamport, from, saved_to });
    context.clipboard.writes.send(clip).map_err(|_| anyhow!("pano iş parçacığı durdu"))
}

/// Okuma görevinin aktarım bildirimleri (başladı/bırakıldı).
pub fn on_notice(context: &mut Context, message: &Message, from: &str) {
    let paused = context.events.snapshot().paused;
    let from = from.to_owned();
    match message {
        Message::ImageStart { total_bytes, .. } if !paused => {
            let bytes = u64::from(*total_bytes);
            context.events.emit(Event::Receiving { content: ContentKind::Image, bytes, items: 1, from });
        }
        Message::FilesStart { files, .. } if !paused => {
            let (bytes, items) = (files.total_bytes(), files.top_level_count());
            context.events.emit(Event::Receiving { content: ContentKind::Files, bytes, items, from });
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
