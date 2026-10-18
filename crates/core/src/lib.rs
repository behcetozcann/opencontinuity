//! G/Ç içermeyen senkron durum makinesi. Kurallar `docs/architecture.md`'de.

use std::collections::{HashSet, VecDeque};

use oc_protocol::{ClipEvent, DeviceId, EventId, Payload};
use sha2::{Digest, Sha256};

const SEEN_EVENTS_CAPACITY: usize = 256;
/// Birden çok eşten art arda gelen içeriklerin yankıları da tanınsın.
const PENDING_ECHO_CAPACITY: usize = 4;

pub struct SyncState {
    device_id: DeviceId,
    clock: u64,
    /// Panodaki içeriğin sürümü: (Lamport, gönderen). Uzak olay ancak bundan
    /// yeniyse uygulanır; böylece üç ve daha çok cihaz da aynı içerikte birleşir.
    current: (u64, DeviceId),
    /// Panoya yazdığımız son uzak içerikler; izleyici onları yerel kopya sanmasın.
    pending_echo: VecDeque<[u8; 32]>,
    seen_order: VecDeque<EventId>,
    seen: HashSet<EventId>,
}

impl SyncState {
    pub fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            clock: 0,
            current: (0, device_id),
            pending_echo: VecDeque::new(),
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
    pub fn on_local(&mut self, payload: Payload) -> Option<ClipEvent> {
        let hash = content_hash(&payload);
        let echo = self.pending_echo.iter().position(|pending| *pending == hash);
        // Eşleşmeden öncekiler izleyicinin hiç görmediği, üzerine yazılmış içeriklerdir.
        // Yankı değilse kullanıcı yeni bir şey kopyalamıştır; bekleyenlerin hepsi geçersiz.
        self.pending_echo.drain(..echo.map_or(self.pending_echo.len(), |index| index + 1));
        if echo.is_some() {
            return None;
        }
        self.clock += 1;
        self.current = (self.clock, self.device_id);
        let event_id = event_id(&self.device_id, self.clock);
        self.remember(event_id);
        Some(ClipEvent { event_id, lamport: self.clock, payload })
    }

    /// Eşten olay geldi. `true` ise içerik panoya yazılmalı.
    pub fn on_remote(&mut self, sender: &DeviceId, event: &ClipEvent) -> bool {
        if !self.remember(event.event_id) {
            return false;
        }
        self.clock = self.clock.max(event.lamport);
        let (current_lamport, current_sender) = self.current;
        let newer = event.lamport > current_lamport || (event.lamport == current_lamport && sender < &current_sender);
        if newer {
            self.current = (event.lamport, *sender);
            self.pending_echo.push_back(content_hash(&event.payload));
            if self.pending_echo.len() > PENDING_ECHO_CAPACITY {
                self.pending_echo.pop_front();
            }
        }
        newer
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

/// Tür de özete girer: aynı baytlı metin ile görsel birbirinin yankısı sayılmaz.
pub fn content_hash(payload: &Payload) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([payload.kind() as u8]);
    hasher.update(payload.digest_input());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests;
