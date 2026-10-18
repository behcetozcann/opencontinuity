//! Protokol v6: `u32` (big-endian) uzunluk + postcard ile kodlanmış [`Message`].
//! Ayrıntı: `docs/protocol.md`.

mod chunks;
mod files;
mod payload;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub use chunks::{Assembler, IMAGE_CHUNK_BYTES, split};
pub use files::{FileEntry, FileList, MAX_FILE_ENTRIES, MAX_FILES_BYTES, portable_name};
pub use payload::{Chunk, ContentKind, PNG_SIGNATURE, Payload};

pub const VERSION: u16 = 6;
pub const MAX_FRAME_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;
/// Görseller parça parça gönderildiği için çerçeve sınırından büyük olabilir.
pub const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;

/// Bir `Peers` mesajındaki en çok cihaz ve alan uzunlukları.
pub const MAX_GROUP_PEERS: usize = 64;
const MAX_PEER_NAME_BYTES: usize = 256;
const MAX_PEER_ADDRESS_BYTES: usize = 64;

pub type DeviceId = [u8; 16];
pub type EventId = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    /// `listen_port`: göndericinin dinlediği port (karşı taraf geri bağlanırken kullanır).
    /// `pairing`: gönderici eşleştirme modunda mı; modlar uyuşmazsa bağlantı kapanır.
    Hello { version: u16, device_id: DeviceId, name: String, listen_port: u16, pairing: bool },
    /// Eşleştirmede kullanıcının kararı; iki uç da `true` göndermeden eş kaydedilmez.
    PairDecision { accepted: bool },
    /// Bağlantı kurulunca gönderilir: göndericinin Lamport saati.
    Clock { lamport: u64 },
    Clip(ClipEvent),
    /// Parçalı görsel olayının başı; ardından toplam `total_bytes` eden `ImageChunk`'lar gelir.
    ImageStart { event_id: EventId, lamport: u64, total_bytes: u32 },
    ImageChunk { data: Chunk },
    /// Dosya aktarımının başı. Ardından listedeki dosyaların içeriği sırayla,
    /// toplam `files.total_bytes()` eden `FileChunk`'lar halinde gelir.
    FilesStart { event_id: EventId, lamport: u64, files: FileList },
    FileChunk { data: Chunk },
    /// Süren görsel/dosya aktarımı bırakıldı (yeni kopya geldi ya da dosya
    /// okunamadı); alan taraf yarım kalanı atar.
    TransferCancel,
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    /// Göndericinin güvendiği cihazlar (grup). Yalnız doğrulanmış eşten kabul edilir;
    /// eşleşme sonrası ve gruba yeni cihaz girince gönderilir.
    Peers { peers: Vec<PeerInfo> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub name: String,
    pub fingerprint: [u8; 32],
    /// Son bilinen `ip:port`; keşif bulamazsa ilk deneme buraya yapılır.
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipEvent {
    pub event_id: EventId,
    pub lamport: u64,
    pub payload: Payload,
}

pub fn encode(message: &Message) -> anyhow::Result<Vec<u8>> {
    validate(message)?;
    let body = postcard::to_stdvec(message)?;
    if body.len() > MAX_FRAME_BYTES {
        bail!("çerçeve çok büyük: {} bayt", body.len());
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

pub fn decode_body(body: &[u8]) -> anyhow::Result<Message> {
    let message: Message = postcard::from_bytes(body).context("bozuk mesaj")?;
    validate(&message)?;
    Ok(message)
}

pub async fn write_message<W: AsyncWrite + Unpin>(writer: &mut W, message: &Message) -> anyhow::Result<()> {
    writer.write_all(&encode(message)?).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> anyhow::Result<Message> {
    let length = reader.read_u32().await? as usize;
    // Uzunluk doğrulanmadan bellek ayrılmaz.
    if length > MAX_FRAME_BYTES {
        bail!("çerçeve çok büyük: {length} bayt");
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await?;
    decode_body(&body)
}

fn validate(message: &Message) -> anyhow::Result<()> {
    match message {
        // Dosya içeriği listeyle aynı mesajda taşınamaz; `FilesStart` + parçalar kullanılır.
        Message::Clip(ClipEvent { payload: Payload::Files(_), .. }) => bail!("dosyalar FilesStart ile gönderilir"),
        Message::Clip(event) => event.payload.validate(),
        Message::ImageStart { total_bytes, .. } if *total_bytes == 0 || *total_bytes as usize > MAX_IMAGE_BYTES => {
            bail!("görsel sınırı aşıldı: {total_bytes} bayt")
        }
        Message::ImageChunk { data } | Message::FileChunk { data } if data.0.is_empty() || data.0.len() > IMAGE_CHUNK_BYTES => {
            bail!("geçersiz parça: {} bayt", data.0.len())
        }
        Message::FilesStart { files, .. } => files.validate(),
        Message::Peers { peers } => validate_peers(peers),
        _ => Ok(()),
    }
}

fn validate_peers(peers: &[PeerInfo]) -> anyhow::Result<()> {
    if peers.len() > MAX_GROUP_PEERS {
        bail!("grupta çok fazla cihaz: {}", peers.len());
    }
    for peer in peers {
        if peer.name.len() > MAX_PEER_NAME_BYTES || peer.address.as_ref().is_some_and(|address| address.len() > MAX_PEER_ADDRESS_BYTES) {
            bail!("geçersiz cihaz kaydı");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
