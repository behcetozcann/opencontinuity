//! Kopyalanan dosya/klasörlerin listesi. Yollar göreli, `/` ayraçlı ve her
//! platformda geçerli adlardan oluşur; alan taraf adları olduğu gibi kullanır,
//! bu yüzden doğrulama hem güvenlik (dizin dışına yazma) hem uyumluluk içindir.

use std::collections::HashSet;

use anyhow::bail;
use serde::{Deserialize, Serialize};

pub const MAX_FILES_BYTES: u64 = 500 * 1024 * 1024;
pub const MAX_FILE_ENTRIES: usize = 5_000;
pub const MAX_PATH_BYTES: usize = 512;

/// Windows'un dosya adında kabul etmediği karakterler (`/` ayraç olduğu için ayrıca).
const RESERVED_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Göreli yol, ör. `Rapor/ek.pdf`. Klasörler içeriklerinden önce gelir.
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileList {
    pub entries: Vec<FileEntry>,
}

impl FileList {
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.size).sum()
    }

    /// Kopyalanan üst düzey öğe sayısı (arayüz "3 öğe" der).
    pub fn top_level_count(&self) -> usize {
        self.entries.iter().filter(|entry| !entry.path.contains('/')).count()
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if self.entries.is_empty() || self.entries.len() > MAX_FILE_ENTRIES {
            bail!("dosya sayısı geçersiz: {}", self.entries.len());
        }
        if self.total_bytes() > MAX_FILES_BYTES {
            bail!("dosya sınırı aşıldı: {} bayt", self.total_bytes());
        }
        let mut directories = HashSet::new();
        let mut seen = HashSet::new();
        for entry in &self.entries {
            if entry.path.len() > MAX_PATH_BYTES {
                bail!("dosya yolu çok uzun");
            }
            if entry.is_dir && entry.size != 0 {
                bail!("klasörün boyutu olamaz");
            }
            if !entry.path.split('/').all(|name| portable_name(name) == name) {
                bail!("geçersiz dosya adı");
            }
            if let Some((parent, _)) = entry.path.rsplit_once('/')
                && !directories.contains(parent)
            {
                bail!("klasör, içeriğinden önce gelmeli");
            }
            // Windows ve varsayılan macOS dosya sistemleri büyük/küçük harf ayırmaz.
            if !seen.insert(entry.path.to_lowercase()) {
                bail!("aynı adlı iki öğe");
            }
            if entry.is_dir {
                directories.insert(entry.path.as_str());
            }
        }
        Ok(())
    }
}

/// Adı her iki platformda geçerli hale getirir; zaten geçerliyse aynen döner.
pub fn portable_name(name: &str) -> String {
    let mut portable: String =
        name.chars().map(|character| if character < ' ' || RESERVED_CHARS.contains(&character) { '_' } else { character }).collect();
    // Windows sondaki nokta ve boşluğu siler; "." ve ".." dizin dışına çıkar.
    let trailing = portable.len() - portable.trim_end_matches(['.', ' ']).len();
    portable.truncate(portable.len() - trailing);
    portable.push_str(&"_".repeat(trailing));
    if portable.is_empty() {
        return "_".to_owned();
    }
    let stem = portable.split('.').next().unwrap_or_default();
    if RESERVED_NAMES.iter().any(|reserved| reserved.eq_ignore_ascii_case(stem)) {
        portable.insert(0, '_');
    }
    portable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(entries: &[(&str, u64, bool)]) -> FileList {
        FileList {
            entries: entries.iter().map(|&(path, size, is_dir)| FileEntry { path: path.to_owned(), size, is_dir }).collect(),
        }
    }

    #[test]
    fn portable_names() {
        assert_eq!(portable_name("Hasta Raporu - Ağustos.pdf"), "Hasta Raporu - Ağustos.pdf");
        assert_eq!(portable_name("a:b?.txt"), "a_b_.txt");
        assert_eq!(portable_name(".."), "__");
        assert_eq!(portable_name("."), "_");
        assert_eq!(portable_name("son. "), "son__");
        assert_eq!(portable_name("con.txt"), "_con.txt");
        assert_eq!(portable_name(""), "_");
        assert_eq!(portable_name(".gizli"), ".gizli");
    }

    #[test]
    fn valid_tree_passes() {
        list(&[("Rapor", 0, true), ("Rapor/ek.pdf", 10, false), ("Rapor/boş", 0, true), ("not.txt", 3, false)]).validate().unwrap();
    }

    #[test]
    fn unsafe_or_inconsistent_lists_are_rejected() {
        let cases: &[&[(&str, u64, bool)]] = &[
            &[],
            &[("../dışarı.txt", 1, false)],
            &[("/etc/passwd", 1, false)],
            &[("a//b", 1, false)],
            &[("C:\\x", 1, false)],
            &[("klasör/dosya", 1, false)],
            &[("Rapor", 5, true)],
            &[("A.txt", 1, false), ("a.txt", 1, false)],
            &[("büyük", MAX_FILES_BYTES + 1, false)],
        ];
        for case in cases {
            assert!(list(case).validate().is_err(), "{case:?} reddedilmeli");
        }
    }

    #[test]
    fn top_level_count_ignores_nested_entries() {
        assert_eq!(list(&[("R", 0, true), ("R/a", 1, false), ("b", 1, false)]).top_level_count(), 2);
    }
}
