//! Protokol v1: `u32` (big-endian) uzunluk + postcard ile kodlanmış [`Message`].
//! Ayrıntı: `docs/protocol.md`.

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;

pub type DeviceId = [u8; 16];
pub type EventId = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Hello { version: u16, device_id: DeviceId, name: String },
    /// Eşleştirmede kullanıcının kararı; iki uç da `true` göndermeden eş kaydedilmez.
    PairDecision { accepted: bool },
    Clip(ClipEvent),
    Ping { nonce: u64 },
    Pong { nonce: u64 },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipEvent {
    pub event_id: EventId,
    pub lamport: u64,
    pub text: String,
}

/// Pano içeriği Debug çıktısında bile görünmesin.
impl std::fmt::Debug for ClipEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClipEvent")
            .field("lamport", &self.lamport)
            .field("text_bytes", &self.text.len())
            .finish_non_exhaustive()
    }
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
    if let Message::Clip(event) = message
        && event.text.len() > MAX_TEXT_BYTES
    {
        bail!("metin sınırı aşıldı: {} bayt", event.text.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
