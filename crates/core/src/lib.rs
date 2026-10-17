//! G/Ç içermeyen senkron durum makinesi. Kurallar `docs/architecture.md`'de.

use std::collections::{HashSet, VecDeque};

use oc_protocol::{ClipEvent, DeviceId, EventId};
use sha2::{Digest, Sha256};

const SEEN_EVENTS_CAPACITY: usize = 256;

pub struct SyncState {
    device_id: DeviceId,
    clock: u64,
    last_local_lamport: u64,
    /// Panoya yazdığımız son uzak içerik; izleyici onu yerel kopya sanmasın.
    pending_echo: Option<[u8; 32]>,
    seen_order: VecDeque<EventId>,
    seen: HashSet<EventId>,
}

impl SyncState {
    pub fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            clock: 0,
            last_local_lamport: 0,
            pending_echo: None,
            seen_order: VecDeque::new(),
            seen: HashSet::new(),
        }
    }

    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Bağlantı kurulunca eşin saati alınır. Yeniden başlayan taraf saatini
    /// 0'dan başlattığı için bu yapılmazsa ilk kopyaları karşıya eski görünür.
    pub fn observe_peer_clock(&mut self, peer_clock: u64) {
        self.clock = self.clock.max(peer_clock);
    }

    /// Yerel pano değişti. Karşıya gönderilecek olayı döner; kendi yazdığımız
    /// uzak içeriğin yankısıysa `None`.
    pub fn on_local_text(&mut self, text: String) -> Option<ClipEvent> {
        if self.pending_echo.take() == Some(content_hash(&text)) {
            return None;
        }
        self.clock += 1;
        self.last_local_lamport = self.clock;
        let event_id = event_id(&self.device_id, self.clock);
        self.remember(event_id);
        Some(ClipEvent { event_id, lamport: self.clock, text })
    }

    /// Eşten olay geldi. `true` ise metin panoya yazılmalı.
    pub fn on_remote(&mut self, sender: &DeviceId, event: &ClipEvent) -> bool {
        if !self.remember(event.event_id) {
            return false;
        }
        self.clock = self.clock.max(event.lamport);
        let newer_than_local_copy = event.lamport > self.last_local_lamport
            || (event.lamport == self.last_local_lamport && sender < &self.device_id);
        if newer_than_local_copy {
            self.pending_echo = Some(content_hash(&event.text));
        }
        newer_than_local_copy
    }

    /// Daha önce görülmemişse kaydeder ve `true` döner.
    fn remember(&mut self, event_id: EventId) -> bool {
        if !self.seen.insert(event_id) {
            return false;
        }
        self.seen_order.push_back(event_id);
        if self.seen_order.len() > SEEN_EVENTS_CAPACITY
            && let Some(oldest) = self.seen_order.pop_front()
        {
            self.seen.remove(&oldest);
        }
        true
    }
}

/// Cihaz başına benzersiz: cihaz kimliğinin ilk 8 baytı + Lamport değeri.
fn event_id(device_id: &DeviceId, lamport: u64) -> EventId {
    let mut id = [0; 16];
    id[..8].copy_from_slice(&device_id[..8]);
    id[8..].copy_from_slice(&lamport.to_be_bytes());
    id
}

pub fn content_hash(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

#[cfg(test)]
mod tests;
