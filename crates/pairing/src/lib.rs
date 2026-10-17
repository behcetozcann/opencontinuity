//! Cihaz kimliği, eş kaydı ve eşleştirme doğrulama kodu.
//!
//! PoC'de anahtar OS anahtarlığı yerine yalnız kullanıcının okuyabildiği
//! dosyada tutulur: imzasız ve her derlemede değişen binary, Keychain'de her
//! seferinde erişim onayı isterdi. Faz 1'de imzalı uygulamayla anahtarlığa geçilir.

mod identity;
mod peer;

pub use identity::{CERT_NAME, Fingerprint, Identity, device_id_of, fingerprint_of};
pub use peer::{Peer, PeerStore};

use sha2::{Digest, Sha256};

/// Doğrulama kodu: `SHA-256("oc-sas-v1" ‖ küçük parmak izi ‖ büyük parmak izi ‖ exporter)`
/// ilk 8 baytı mod 10⁶. İki uç da aynı sonucu bulur; araya giren saldırgan
/// iki ayrı TLS oturumu kurmak zorunda kalır ve kodlar tutmaz.
pub fn sas_code(first: &Fingerprint, second: &Fingerprint, exporter: &[u8; 32]) -> String {
    let (low, high) = if first <= second { (first, second) } else { (second, first) };
    let mut hasher = Sha256::new();
    hasher.update(b"oc-sas-v1");
    hasher.update(low);
    hasher.update(high);
    hasher.update(exporter);
    let digest = hasher.finalize();
    let mut prefix = [0; 8];
    prefix.copy_from_slice(&digest[..8]);
    let code = u64::from_be_bytes(prefix) % 1_000_000;
    format!("{:03} {:03}", code / 1000, code % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sas_is_symmetric() {
        let (a, b, exporter) = ([1; 32], [2; 32], [9; 32]);
        assert_eq!(sas_code(&a, &b, &exporter), sas_code(&b, &a, &exporter));
    }

    #[test]
    fn sas_changes_with_exporter_or_fingerprint() {
        let (a, b) = ([1; 32], [2; 32]);
        let base = sas_code(&a, &b, &[9; 32]);
        assert_ne!(base, sas_code(&a, &b, &[8; 32]));
        assert_ne!(base, sas_code(&a, &[3; 32], &[9; 32]));
    }

    #[test]
    fn sas_has_six_digits() {
        let code = sas_code(&[0; 32], &[0; 32], &[0; 32]);
        assert_eq!(code.len(), 7);
        assert!(code.chars().filter(|c| *c != ' ').all(|c| c.is_ascii_digit()));
    }
}
