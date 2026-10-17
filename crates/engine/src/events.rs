//! Motorun dışarıya bildirdiği olaylar. Pano içeriği hiçbir olaya girmez;
//! yalnız bayt sayısı, Lamport değeri ve teşhis metni.

use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PeerSummary {
    pub name: String,
    pub fingerprint_prefix: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum PairingStage {
    /// Aynı ağda eşleştirme modundaki cihaz aranıyor; bu sırada bağlantı da kabul edilir.
    Searching { local_ip: Option<String> },
    /// Kullanıcı iki ekrandaki kodu karşılaştırmalı.
    Confirm { peer: String, code: String },
    /// Biz onayladık, karşı cihazın onayı bekleniyor.
    WaitingPeer { peer: String },
}

/// Arayüzün tek başına çizebileceği anlık durum; her değişimde tamamı gönderilir.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Snapshot {
    pub device_name: String,
    pub fingerprint_prefix: String,
    pub peer: Option<PeerSummary>,
    pub connected_ip: Option<String>,
    pub paused: bool,
    pub pairing: Option<PairingStage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    State(Snapshot),
    Sent { bytes: usize, lamport: u64 },
    Received { bytes: usize, lamport: u64 },
    Skipped { reason: String },
    Disconnected { reason: String },
    Paired { peer: String },
    PairingFailed { reason: String },
    Diagnostic { message: String },
}

pub(crate) struct Emitter {
    sender: UnboundedSender<Event>,
    snapshot: Snapshot,
}

impl Emitter {
    pub fn new(sender: UnboundedSender<Event>) -> Self {
        Self { sender, snapshot: Snapshot::default() }
    }

    pub fn sender(&self) -> UnboundedSender<Event> {
        self.sender.clone()
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn emit(&self, event: Event) {
        // Dinleyen kalmadıysa motor yine de çalışmayı sürdürür.
        let _ = self.sender.send(event);
    }

    pub fn diagnostic(&self, message: impl Into<String>) {
        self.emit(Event::Diagnostic { message: message.into() });
    }

    /// Durumu değiştirir; gerçekten değiştiyse yeni durumu yayınlar.
    pub fn update(&mut self, change: impl FnOnce(&mut Snapshot)) {
        let before = self.snapshot.clone();
        change(&mut self.snapshot);
        if self.snapshot != before {
            self.emit(Event::State(self.snapshot.clone()));
        }
    }
}
