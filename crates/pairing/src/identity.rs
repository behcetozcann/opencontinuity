use std::fs;
use std::path::Path;

use anyhow::Context;
use oc_protocol::DeviceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type Fingerprint = [u8; 32];

const IDENTITY_FILE: &str = "identity.json";
/// Sertifikadaki ad; doğrulamada kullanılmaz, güven parmak izine dayanır.
pub const CERT_NAME: &str = "opencontinuity.local";

/// Cihazın kalıcı self-signed P-256 sertifikası ve özel anahtarı.
#[derive(Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    #[serde(with = "hex")]
    pub cert_der: Vec<u8>,
    #[serde(with = "hex")]
    pub key_der: Vec<u8>,
}

impl Identity {
    pub fn generate(name: String) -> anyhow::Result<Self> {
        let certified = rcgen::generate_simple_self_signed(vec![CERT_NAME.to_owned()])?;
        Ok(Self {
            name,
            cert_der: certified.cert.der().to_vec(),
            key_der: certified.signing_key.serialize_der(),
        })
    }

    pub fn load_or_create(dir: &Path, name: &str) -> anyhow::Result<Self> {
        let path = dir.join(IDENTITY_FILE);
        if path.exists() {
            let json = fs::read_to_string(&path).with_context(|| format!("{} okunamadı", path.display()))?;
            return serde_json::from_str(&json).context("kimlik dosyası bozuk");
        }
        let identity = Self::generate(name.to_owned())?;
        fs::create_dir_all(dir)?;
        write_private(&path, &serde_json::to_vec_pretty(&identity)?)?;
        Ok(identity)
    }

    pub fn fingerprint(&self) -> Fingerprint {
        fingerprint_of(&self.cert_der)
    }

    pub fn device_id(&self) -> DeviceId {
        device_id_of(&self.fingerprint())
    }
}

pub fn fingerprint_of(cert_der: &[u8]) -> Fingerprint {
    Sha256::digest(cert_der).into()
}

pub fn device_id_of(fingerprint: &Fingerprint) -> DeviceId {
    let mut id = [0; 16];
    id.copy_from_slice(&fingerprint[..16]);
    id
}

/// Unix'te yalnız sahibi okuyabilir (0600). Windows'ta kullanıcı profili ACL'i geçerli.
fn write_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)?.write_all(bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_persists_across_loads() {
        let dir = tempfile::tempdir().unwrap();
        let first = Identity::load_or_create(dir.path(), "Mac").unwrap();
        let second = Identity::load_or_create(dir.path(), "başka ad").unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(second.name, "Mac");
    }

    #[cfg(unix)]
    #[test]
    fn identity_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        Identity::load_or_create(dir.path(), "Mac").unwrap();
        let mode = fs::metadata(dir.path().join(IDENTITY_FILE)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
