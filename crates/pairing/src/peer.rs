use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::Fingerprint;

const PEERS_FILE: &str = "peers.json";
/// 0.1.7 ve öncesinin tek eş kaydı; ilk yüklemede `peers.json`'a taşınır.
const LEGACY_PEER_FILE: &str = "peer.json";

/// Eşleşmiş ya da gruptaki bir eşin tanıttığı cihaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    pub name: String,
    #[serde(with = "hex")]
    pub fingerprint: Fingerprint,
    pub last_address: Option<String>,
}

/// `removed`: kullanıcının kaldırdığı cihazlar; başka bir eş onları tanıtsa da
/// eklenmez. Yeniden eşleştirme (kod onayı) kaydı siler.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PeerFile {
    peers: Vec<Peer>,
    #[serde(default, with = "fingerprints")]
    removed: Vec<Fingerprint>,
}

mod fingerprints {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::Fingerprint;

    pub fn serialize<S: Serializer>(list: &[Fingerprint], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(list.iter().map(hex::encode))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Fingerprint>, D::Error> {
        let texts = Vec::<String>::deserialize(deserializer)?;
        texts
            .iter()
            .map(|text| {
                let mut fingerprint = [0; 32];
                hex::decode_to_slice(text, &mut fingerprint).map_err(serde::de::Error::custom)?;
                Ok(fingerprint)
            })
            .collect()
    }
}

pub struct PeerStore {
    path: PathBuf,
    legacy_path: PathBuf,
}

impl PeerStore {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join(PEERS_FILE), legacy_path: dir.join(LEGACY_PEER_FILE) }
    }

    pub fn load_all(&self) -> anyhow::Result<Vec<Peer>> {
        Ok(self.read()?.peers)
    }

    /// Kod onayıyla eşleşen cihaz: kaydı ekler ya da günceller, kaldırılmışsa geri alır.
    pub fn upsert(&self, peer: &Peer) -> anyhow::Result<()> {
        let mut file = self.read()?;
        file.removed.retain(|removed| removed != &peer.fingerprint);
        match file.peers.iter_mut().find(|existing| existing.fingerprint == peer.fingerprint) {
            Some(existing) => *existing = peer.clone(),
            None => file.peers.push(peer.clone()),
        }
        self.write(&file)
    }

    /// Gruptaki bir eşin tanıttığı cihaz: bilinmiyor ve kaldırılmamışsa eklenir.
    /// Eklendiyse `true`.
    pub fn add_introduced(&self, peer: &Peer) -> anyhow::Result<bool> {
        let mut file = self.read()?;
        if file.removed.contains(&peer.fingerprint) || file.peers.iter().any(|existing| existing.fingerprint == peer.fingerprint) {
            return Ok(false);
        }
        file.peers.push(peer.clone());
        self.write(&file)?;
        Ok(true)
    }

    /// Kayıtlıysa siler, tanıtılınca geri gelmemesi için hatırlar ve `true` döner.
    pub fn remove(&self, fingerprint: &Fingerprint) -> anyhow::Result<bool> {
        let mut file = self.read()?;
        let before = file.peers.len();
        file.peers.retain(|peer| &peer.fingerprint != fingerprint);
        if file.peers.len() == before {
            return Ok(false);
        }
        file.removed.push(*fingerprint);
        self.write(&file)?;
        Ok(true)
    }

    fn read(&self) -> anyhow::Result<PeerFile> {
        let mut file = if self.path.exists() {
            serde_json::from_str(&fs::read_to_string(&self.path)?).context("eş dosyası bozuk")?
        } else {
            PeerFile::default()
        };
        if self.legacy_path.exists() {
            let legacy: Peer =
                serde_json::from_str(&fs::read_to_string(&self.legacy_path)?).context("eski eş dosyası bozuk")?;
            if !file.peers.iter().any(|peer| peer.fingerprint == legacy.fingerprint) {
                file.peers.push(legacy);
            }
            self.write(&file)?;
            fs::remove_file(&self.legacy_path)?;
        }
        Ok(file)
    }

    fn write(&self, file: &PeerFile) -> anyhow::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        // Yarım yazılmış dosya tüm grubu kaybettirmesin.
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(file)?)?;
        fs::rename(&temporary, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(name: &str, byte: u8) -> Peer {
        Peer { name: name.into(), fingerprint: [byte; 32], last_address: Some("192.168.1.5:47800".into()) }
    }

    #[test]
    fn upsert_load_remove() {
        let dir = tempfile::tempdir().unwrap();
        let store = PeerStore::new(dir.path());
        assert!(store.load_all().unwrap().is_empty());
        store.upsert(&peer("Windows", 5)).unwrap();
        store.upsert(&peer("Linux", 6)).unwrap();
        let mut renamed = peer("Windows PC", 5);
        renamed.last_address = None;
        store.upsert(&renamed).unwrap();
        assert_eq!(store.load_all().unwrap(), vec![renamed, peer("Linux", 6)]);
        assert!(store.remove(&[5; 32]).unwrap());
        assert!(!store.remove(&[5; 32]).unwrap());
        assert_eq!(store.load_all().unwrap(), vec![peer("Linux", 6)]);
    }

    #[test]
    fn removed_peer_is_not_reintroduced_until_paired_again() {
        let dir = tempfile::tempdir().unwrap();
        let store = PeerStore::new(dir.path());
        assert!(store.add_introduced(&peer("Linux", 6)).unwrap());
        assert!(!store.add_introduced(&peer("Linux", 6)).unwrap(), "zaten kayıtlı");
        assert!(store.remove(&[6; 32]).unwrap());
        assert!(!store.add_introduced(&peer("Linux", 6)).unwrap(), "kaldırılan geri gelmemeli");
        assert!(store.load_all().unwrap().is_empty());
        store.upsert(&peer("Linux", 6)).unwrap();
        assert_eq!(store.load_all().unwrap(), vec![peer("Linux", 6)]);
    }

    #[test]
    fn legacy_single_peer_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = peer("Windows", 5);
        fs::write(dir.path().join(LEGACY_PEER_FILE), serde_json::to_vec(&legacy).unwrap()).unwrap();
        let store = PeerStore::new(dir.path());
        assert_eq!(store.load_all().unwrap(), vec![legacy.clone()]);
        assert!(!dir.path().join(LEGACY_PEER_FILE).exists());
        assert_eq!(store.load_all().unwrap(), vec![legacy]);
    }
}
