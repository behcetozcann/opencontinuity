//! `oc run`: motoru çalıştırır ve olayları zaman damgasıyla yazdırır.

use std::sync::OnceLock;
use std::time::Instant;

use anyhow::bail;
use oc_engine::{ClipboardSource, Engine, EngineConfig, Event, Snapshot};
use oc_pairing::PeerStore;

pub async fn run() -> anyhow::Result<()> {
    let config = EngineConfig::system()?;
    if PeerStore::new(&config.config_dir).load_all()?.is_empty() {
        bail!("eşleşmiş cihaz yok; önce `oc pair` çalıştır");
    }
    let (engine, _handle, mut events) = Engine::new(config, ClipboardSource::System);
    let task = tokio::spawn(engine.run());
    let mut previous = Snapshot::default();
    while let Some(event) = events.recv().await {
        print_event(&event, &mut previous);
    }
    task.await?
}

fn print_event(event: &Event, previous: &mut Snapshot) {
    match event {
        Event::State(snapshot) => {
            if snapshot.connected_count() != previous.connected_count() {
                log(format!("Bağlı cihaz: {}/{}", snapshot.connected_count(), snapshot.peers.len()));
            }
            if snapshot.paused != previous.paused {
                log(if snapshot.paused { "Duraklatıldı." } else { "Devam ediyor." }.into());
            }
            *previous = snapshot.clone();
        }
        Event::Sending { content, bytes, items } => log(format!("Gönderiliyor: {content:?}, {items} öğe, {bytes} bayt")),
        Event::Receiving { content, bytes, items, from } => log(format!("Alınıyor ({from}): {content:?}, {items} öğe, {bytes} bayt")),
        Event::TooLarge { content, bytes, limit } => log(format!("Çok büyük, gönderilmedi: {content:?}, {bytes} bayt (sınır {limit})")),
        Event::Sent { content, bytes, items, lamport } => log(format!("Gönderildi: {content:?}, {items} öğe, {bytes} bayt (lamport {lamport})")),
        Event::Received { content, bytes, items, lamport, from, .. } => {
            log(format!("Alındı ({from}): {content:?}, {items} öğe, {bytes} bayt (lamport {lamport})"))
        }
        Event::Skipped { reason } => log(reason.clone()),
        Event::Disconnected { peer, reason } => log(format!("Bağlantı koptu ({peer}): {reason}")),
        Event::Paired { peer } => log(format!("Eşleşti: {peer}")),
        Event::PeerAdded { peer, via } => log(format!("Gruba eklendi: {peer} ({via} tanıttı)")),
        Event::PairingFailed { reason } => log(format!("Eşleştirme başarısız: {reason}")),
        Event::Diagnostic { message } => log(message.clone()),
    }
}

pub fn log(message: String) {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed().as_secs_f32();
    println!("[{elapsed:7.1} sn] {message}");
}
