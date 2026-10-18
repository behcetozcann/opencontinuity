use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{MAX_IMAGE_BYTES, MAX_TEXT_BYTES};

pub const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Pano içeriği. Debug çıktısında yalnız tür ve uzunluk görünür.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    Text(String),
    /// PNG dosyasının baytları.
    Png(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    Text,
    Image,
}

impl Payload {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Payload::Text(text) => text.as_bytes(),
            Payload::Png(png) => png,
        }
    }

    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    pub fn kind(&self) -> ContentKind {
        match self {
            Payload::Text(_) => ContentKind::Text,
            Payload::Png(_) => ContentKind::Image,
        }
    }

    pub fn max_bytes(&self) -> usize {
        match self {
            Payload::Text(_) => MAX_TEXT_BYTES,
            Payload::Png(_) => MAX_IMAGE_BYTES,
        }
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if self.len() > self.max_bytes() {
            bail!("içerik sınırı aşıldı: {} bayt", self.len());
        }
        if let Payload::Png(png) = self
            && !png.starts_with(&PNG_SIGNATURE)
        {
            bail!("görsel PNG değil");
        }
        Ok(())
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
