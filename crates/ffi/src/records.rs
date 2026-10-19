//! Motor türlerinin Swift'e geçen kopyaları. Pano içeriği yalnız
//! `ClipRecord`'da taşınır; olaylarda bayt sayısı ve adlar var.

use oc_engine::{Clip, ContentKind, Event, PairingStage, PeerSummary, Snapshot};

#[derive(uniffi::Record)]
pub struct PeerRecord {
    pub name: String,
    pub fingerprint_prefix: String,
    pub address: Option<String>,
    pub connected: bool,
    pub link_error: Option<String>,
}

#[derive(uniffi::Enum)]
pub enum PairingRecord {
    Searching { local_ip: Option<String> },
    Confirm { peer: String, code: String },
    WaitingPeer { peer: String },
}

#[derive(uniffi::Record)]
pub struct SnapshotRecord {
    pub device_name: String,
    pub fingerprint_prefix: String,
    pub peers: Vec<PeerRecord>,
    pub paused: bool,
    pub pairing: Option<PairingRecord>,
}

#[derive(uniffi::Enum)]
pub enum ContentKindRecord {
    Text,
    Image,
    Files,
}

#[derive(uniffi::Enum)]
pub enum EventRecord {
    Sending { content: ContentKindRecord, bytes: u64, items: u64 },
    Receiving { content: ContentKindRecord, bytes: u64, items: u64, from: String },
    Sent { content: ContentKindRecord, bytes: u64, items: u64 },
    Received { content: ContentKindRecord, bytes: u64, items: u64, from: String, saved_to: Option<String> },
    TooLarge { content: ContentKindRecord, bytes: u64, limit: u64 },
    Skipped { reason: String },
    Disconnected { peer: String, reason: String },
    Paired { peer: String },
    PeerAdded { peer: String, via: String },
    PairingFailed { reason: String },
    Diagnostic { message: String },
}

#[derive(uniffi::Enum)]
pub enum ClipRecord {
    Text { text: String },
    Png { data: Vec<u8> },
    Files { paths: Vec<String> },
}

impl From<Snapshot> for SnapshotRecord {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            device_name: snapshot.device_name,
            fingerprint_prefix: snapshot.fingerprint_prefix,
            peers: snapshot.peers.into_iter().map(PeerRecord::from).collect(),
            paused: snapshot.paused,
            pairing: snapshot.pairing.map(PairingRecord::from),
        }
    }
}

impl From<PeerSummary> for PeerRecord {
    fn from(peer: PeerSummary) -> Self {
        Self {
            name: peer.name,
            fingerprint_prefix: peer.fingerprint_prefix,
            address: peer.address,
            connected: peer.connected,
            link_error: peer.link_error,
        }
    }
}

impl From<PairingStage> for PairingRecord {
    fn from(stage: PairingStage) -> Self {
        match stage {
            PairingStage::Searching { local_ip } => Self::Searching { local_ip },
            PairingStage::Confirm { peer, code } => Self::Confirm { peer, code },
            PairingStage::WaitingPeer { peer } => Self::WaitingPeer { peer },
        }
    }
}

impl From<ContentKind> for ContentKindRecord {
    fn from(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Text => Self::Text,
            ContentKind::Image => Self::Image,
            ContentKind::Files => Self::Files,
        }
    }
}

impl EventRecord {
    /// `State` ayrı geri çağrıyla gider; burada `None` döner.
    pub fn from_event(event: Event) -> Option<Self> {
        Some(match event {
            Event::State(_) => return None,
            Event::Sending { content, bytes, items } => Self::Sending { content: content.into(), bytes, items: items as u64 },
            Event::Receiving { content, bytes, items, from } => {
                Self::Receiving { content: content.into(), bytes, items: items as u64, from }
            }
            Event::Sent { content, bytes, items, .. } => Self::Sent { content: content.into(), bytes, items: items as u64 },
            Event::Received { content, bytes, items, from, saved_to, .. } => Self::Received {
                content: content.into(),
                bytes,
                items: items as u64,
                from,
                saved_to: saved_to.map(|path| path.to_string_lossy().into_owned()),
            },
            Event::TooLarge { content, bytes, limit } => Self::TooLarge { content: content.into(), bytes, limit },
            Event::Skipped { reason } => Self::Skipped { reason },
            Event::Disconnected { peer, reason } => Self::Disconnected { peer, reason },
            Event::Paired { peer } => Self::Paired { peer },
            Event::PeerAdded { peer, via } => Self::PeerAdded { peer, via },
            Event::PairingFailed { reason } => Self::PairingFailed { reason },
            Event::Diagnostic { message } => Self::Diagnostic { message },
        })
    }
}

impl From<Clip> for ClipRecord {
    fn from(clip: Clip) -> Self {
        match clip {
            Clip::Text(text) => Self::Text { text },
            Clip::Png(data) => Self::Png { data },
            Clip::Files(paths) => Self::Files { paths: paths.iter().map(|path| path.to_string_lossy().into_owned()).collect() },
        }
    }
}
