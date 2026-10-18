use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{FileList, MAX_FILES_BYTES, MAX_IMAGE_BYTES, MAX_TEXT_BYTES};

pub const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Pano içeriği. Debug çıktısında yalnız tür ve uzunluk görünür.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    Text(String),
    /// PNG dosyasının baytları.
    Png(Vec<u8>),
    /// Dosya/klasör listesi; içerik ayrıca `FileChunk`'larla akar.
    Files(FileList),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    Text,
    Image,
    Files,
}

impl Payload {
    /// Metin ve görselde içerik baytları, dosyada toplam boyut.
    pub fn len(&self) -> u64 {
        match self {
            Payload::Text(text) => text.len() as u64,
            Payload::Png(png) => png.len() as u64,
            Payload::Files(files) => files.total_bytes(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Yankı karşılaştırması için özetlenen baytlar: dosyada liste (ad, boyut, yapı).
    pub fn digest_input(&self) -> std::borrow::Cow<'_, [u8]> {
        match self {
            Payload::Text(text) => text.as_bytes().into(),
            Payload::Png(png) => png.as_slice().into(),
            Payload::Files(files) => postcard::to_stdvec(files).unwrap_or_default().into(),
        }
    }

    pub fn kind(&self) -> ContentKind {
        match self {
            Payload::Text(_) => ContentKind::Text,
            Payload::Png(_) => ContentKind::Image,
            Payload::Files(_) => ContentKind::Files,
        }
    }

    pub fn max_bytes(&self) -> u64 {
        match self {
            Payload::Text(_) => MAX_TEXT_BYTES as u64,
            Payload::Png(_) => MAX_IMAGE_BYTES as u64,
            Payload::Files(_) => MAX_FILES_BYTES,
        }
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if self.len() > self.max_bytes() {
            bail!("içerik sınırı aşıldı: {} bayt", self.len());
        }
        match self {
            Payload::Png(png) if !png.starts_with(&PNG_SIGNATURE) => bail!("görsel PNG değil"),
            Payload::Files(files) => files.validate(),
            _ => Ok(()),
        }
    }
}

impl std::fmt::Debug for Payload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}({} bayt)", self.kind(), self.len())
    }
}

/// Görsel parçasının baytları; Debug çıktısında yalnız uzunluk.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk(pub Vec<u8>);

impl std::fmt::Debug for Chunk {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Chunk({} bayt)", self.0.len())
    }
}
