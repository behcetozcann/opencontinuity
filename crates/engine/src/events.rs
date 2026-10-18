//! Motorun dışarıya bildirdiği olaylar. Pano içeriği hiçbir olaya girmez;
//! yalnız bayt sayısı, Lamport değeri ve teşhis metni.

use std::path::PathBuf;

use oc_protocol::ContentKind;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PeerSummary {
    pub name: String,
    pub fingerprint_prefix: String,
    /// Bağlıysa bağlantının adresi, değilse son bilinen adres.
    pub address: Option<String>,
    pub connected: bool,
    /// Son bağlanma denemesinin hatası; bağlanınca temizlenir.
    pub link_error: Option<String>,
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
    /// Gruptaki diğer cihazlar; boşsa eşleşme yok.
    pub peers: Vec<PeerSummary>,
    pub paused: bool,
    pub pairing: Option<PairingStage>,
}

impl Snapshot {
    pub fn connected_count(&self) -> usize {
        self.peers.iter().filter(|peer| peer.connected).count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    State(Snapshot),
    /// Görsel/dosya gönderimi başladı; sonunda `Sent`, `Skipped` ya da `Disconnected`.
    /// `items`: dosyada kopyalanan üst düzey öğe sayısı, diğerlerinde 1.
    Sending { content: ContentKind, bytes: u64, items: usize },
    /// Eşten görsel/dosya gelmeye başladı; sonunda `Received`, `Skipped` ya da `Disconnected`.
    Receiving { content: ContentKind, bytes: u64, items: usize, from: String },
    /// Kopya bir eşe ulaştı; aynı kopya için bir kez (ilk teslimde) yayınlanır.
    Sent { content: ContentKind, bytes: u64, items: usize, lamport: u64 },
    /// `saved_to`: dosyalar geldiyse kaydedildikleri klasör. `from`: gönderen cihazın adı.
    Received { content: ContentKind, bytes: u64, items: usize, lamport: u64, from: String, saved_to: Option<PathBuf> },
    /// Kopya sınırı aştığı için gönderilmedi.
    TooLarge { content: ContentKind, bytes: u64, limit: u64 },
    Skipped { reason: String },
    Disconnected { peer: String, reason: String },
    Paired { peer: String },
    /// Gruptaki bir eş yeni bir cihazı tanıttı.
    PeerAdded { peer: String, via: String },
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
