use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::Fingerprint;

const PEER_FILE: &str = "peer.json";

/// Eşleşmiş tek eş (V0.1 tek eş destekler).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    pub name: String,
    #[serde(with = "hex")]
    pub fingerprint: Fingerprint,
    pub last_address: Option<String>,
}

pub struct PeerStore {
    path: PathBuf,
}

impl PeerStore {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join(PEER_FILE) }
    }

    pub fn load(&self) -> anyhow::Result<Option<Peer>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&self.path)?;
        Ok(Some(serde_json::from_str(&json).context("eş dosyası bozuk")?))
    }

    pub fn save(&self, peer: &Peer) -> anyhow::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&self.path, serde_json::to_vec_pretty(peer)?)?;
        Ok(())
    }

    /// Eş kayıtlıysa siler ve `true` döner.
    pub fn remove(&self) -> anyhow::Result<bool> {
        if !self.path.exists() {
            return Ok(false);
        }
        fs::remove_file(&self.path)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_remove() {
        let dir = tempfile::tempdir().unwrap();
        let store = PeerStore::new(dir.path());
        assert_eq!(store.load().unwrap(), None);
        let peer = Peer { name: "Windows".into(), fingerprint: [5; 32], last_address: Some("192.168.1.5:47800".into()) };
        store.save(&peer).unwrap();
        assert_eq!(store.load().unwrap(), Some(peer));
        assert!(store.remove().unwrap());
        assert!(!store.remove().unwrap());
        assert_eq!(store.load().unwrap(), None);
    }
}
