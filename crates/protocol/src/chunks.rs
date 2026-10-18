//! Görseller çerçeve sınırını aşabildiği için `ImageStart` + `ImageChunk`
//! dizisi olarak gönderilir; alan taraf tek `Clip` olayına birleştirir.

use anyhow::bail;

use crate::{Chunk, ClipEvent, EventId, Message, Payload};

pub const IMAGE_CHUNK_BYTES: usize = 256 * 1024;

/// Görsel olayını parçalara böler; diğer mesajlar olduğu gibi döner.
pub fn split(message: Message) -> Vec<Message> {
    let Message::Clip(ClipEvent { event_id, lamport, payload: Payload::Png(png) }) = message else {
        return vec![message];
    };
    let mut messages = Vec::with_capacity(1 + png.len().div_ceil(IMAGE_CHUNK_BYTES));
    messages.push(Message::ImageStart { event_id, lamport, total_bytes: png.len() as u32 });
    messages.extend(png.chunks(IMAGE_CHUNK_BYTES).map(|data| Message::ImageChunk { data: Chunk(data.to_vec()) }));
    messages
}

struct Pending {
    event_id: EventId,
    lamport: u64,
    total: usize,
    data: Vec<u8>,
}

/// Bağlantı başına bir tane; bağlantı koparsa yarım görsel onunla birlikte düşer.
#[derive(Default)]
pub struct Assembler {
    pending: Option<Pending>,
}

impl Assembler {
    /// `TransferCancel` geldi: yarım görsel atılır.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// Tamamlanan mesajı döner; görsel parçası bekleniyorsa `None`.
    pub fn push(&mut self, message: Message) -> anyhow::Result<Option<Message>> {
        match message {
            Message::ImageStart { event_id, lamport, total_bytes } => {
                if self.pending.is_some() {
                    bail!("önceki görsel tamamlanmadan yeni görsel başladı");
                }
                // total_bytes `validate` ile MAX_IMAGE_BYTES'a sınırlandı.
                let total = total_bytes as usize;
                self.pending = Some(Pending { event_id, lamport, total, data: Vec::with_capacity(total) });
                Ok(None)
            }
            Message::ImageChunk { data } => {
                let Some(pending) = self.pending.as_mut() else { bail!("başlangıcı olmayan görsel parçası") };
                if pending.data.len() + data.0.len() > pending.total {
                    bail!("görsel parçaları bildirilen boyutu aştı");
                }
                pending.data.extend_from_slice(&data.0);
                if pending.data.len() < pending.total {
                    return Ok(None);
                }
                let Pending { event_id, lamport, data, .. } = self.pending.take().expect("yukarıda var");
                let payload = Payload::Png(data);
                payload.validate()?;
                Ok(Some(Message::Clip(ClipEvent { event_id, lamport, payload })))
            }
            other => Ok(Some(other)),
        }
    }
}
