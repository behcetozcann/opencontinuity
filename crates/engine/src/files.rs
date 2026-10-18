//! Dosya aktarımının disk tarafı. Gönderen kopyalanan öğeleri gezip listeyi
//! çıkarır; alan parçaları `<kök>/.partial-<olay>` altına yazar, kabul edilince
//! klasörü tarihli ada çevirir ve panoya o yolları koyar.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use oc_protocol::{EventId, FileEntry, FileList, MAX_FILE_ENTRIES, MAX_FILES_BYTES, portable_name};
use tokio::io::AsyncWriteExt;

/// İşletim sisteminin klasörlere bıraktığı, karşıya taşımaya değmeyen dosyalar.
const SKIPPED_NAMES: [&str; 3] = [".DS_Store", "Thumbs.db", "desktop.ini"];
/// İndirilenler'de tutulan son aktarım klasörü sayısı; eskiler silinir.
const KEPT_TRANSFERS: usize = 10;
const PARTIAL_PREFIX: &str = ".partial-";

pub struct Outgoing {
    pub list: FileList,
    /// Listedeki dosyaların (klasörler hariç) kaynak yolları, liste sırasıyla.
    pub sources: Vec<PathBuf>,
}

pub enum Collected {
    Ready(Outgoing),
    /// Sınır aşıldı; `bytes` en az bu kadar (gezinti sınırda durur).
    TooLarge { bytes: u64 },
}

/// Kopyalanan öğeleri gezer. Sembolik bağlar izlenmez; klasör içeriği ada göre sıralanır
/// ki iki uç aynı listeyi üretsin (yankı karşılaştırması buna dayanır).
pub fn collect(roots: &[PathBuf]) -> anyhow::Result<Collected> {
    let mut outgoing = Outgoing { list: FileList::default(), sources: Vec::new() };
    let mut total = 0;
    for root in roots {
        let name = root.file_name().context("kopyalanan öğenin adı yok")?.to_string_lossy();
        if !walk(root, portable_name(&name), &mut outgoing, &mut total)? {
            return Ok(Collected::TooLarge { bytes: total });
        }
    }
    if outgoing.list.entries.is_empty() {
        bail!("kopyalananlar arasında gönderilecek dosya yok");
    }
    Ok(Collected::Ready(outgoing))
}

/// Sınır aşılırsa `false`.
fn walk(path: &Path, relative: String, outgoing: &mut Outgoing, total: &mut u64) -> anyhow::Result<bool> {
    let metadata = std::fs::symlink_metadata(path).with_context(|| format!("{} okunamadı", path.display()))?;
    if metadata.is_symlink() {
        return Ok(true);
    }
    if outgoing.list.entries.len() >= MAX_FILE_ENTRIES {
        return Ok(false);
    }
    if metadata.is_file() {
        *total += metadata.len();
        if *total > MAX_FILES_BYTES {
            return Ok(false);
        }
        outgoing.list.entries.push(FileEntry { path: relative, size: metadata.len(), is_dir: false });
        outgoing.sources.push(path.to_path_buf());
        return Ok(true);
    }
    outgoing.list.entries.push(FileEntry { path: relative.clone(), size: 0, is_dir: true });
    let mut children: Vec<_> = std::fs::read_dir(path)?.collect::<Result<_, _>>()?;
    children.sort_by_key(|child| child.file_name());
    for child in children {
        let name = child.file_name().to_string_lossy().into_owned();
        if SKIPPED_NAMES.contains(&name.as_str()) {
            continue;
        }
        if !walk(&child.path(), format!("{relative}/{}", portable_name(&name)), outgoing, total)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn partial_dir(root: &Path, event_id: &EventId) -> PathBuf {
    root.join(format!("{PARTIAL_PREFIX}{}", hex::encode(event_id)))
}

/// Gelen dosyaları sırayla diske yazar. Bağlantı başına bir tane.
pub struct FileSink {
    root: PathBuf,
    current: Option<Incoming>,
}

struct Incoming {
    event_id: EventId,
    lamport: u64,
    list: FileList,
    dir: PathBuf,
    /// Sıradaki yazılacak girdi.
    next: usize,
    file: Option<(tokio::fs::File, u64)>,
}

pub struct Completed {
    pub event_id: EventId,
    pub lamport: u64,
    pub list: FileList,
}

impl FileSink {
    pub fn new(root: PathBuf) -> Self {
        Self { root, current: None }
    }

    /// Liste doğrulanmış gelir (`oc_protocol::validate`). Boş dosyalardan oluşan
    /// liste hemen tamamlanabilir.
    pub async fn start(&mut self, event_id: EventId, lamport: u64, list: FileList) -> anyhow::Result<Option<Completed>> {
        if self.current.is_some() {
            bail!("önceki dosya aktarımı tamamlanmadan yenisi başladı");
        }
        let dir = partial_dir(&self.root, &event_id);
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.with_context(|| format!("{} oluşturulamadı", dir.display()))?;
        self.current = Some(Incoming { event_id, lamport, list, dir, next: 0, file: None });
        self.advance().await
    }

    pub async fn chunk(&mut self, data: &[u8]) -> anyhow::Result<Option<Completed>> {
        let Some(incoming) = self.current.as_mut() else { bail!("başlangıcı olmayan dosya parçası") };
        let Some((file, remaining)) = incoming.file.as_mut() else { bail!("dosya parçaları bildirilen boyutu aştı") };
        if data.len() as u64 > *remaining {
            bail!("dosya parçaları bildirilen boyutu aştı");
        }
        file.write_all(data).await?;
        *remaining -= data.len() as u64;
        if *remaining == 0 {
            file.flush().await?;
            incoming.file = None;
            return self.advance().await;
        }
        Ok(None)
    }

    pub async fn cancel(&mut self) {
        if let Some(incoming) = self.current.take() {
            let _ = tokio::fs::remove_dir_all(incoming.dir).await;
        }
    }

    /// Sıradaki veri bekleyen dosyayı açar; klasörleri ve boş dosyaları yolda oluşturur.
    async fn advance(&mut self) -> anyhow::Result<Option<Completed>> {
        let incoming = self.current.as_mut().expect("aktarım sürüyor");
        while let Some(entry) = incoming.list.entries.get(incoming.next) {
            incoming.next += 1;
            let path = incoming.dir.join(entry.path.split('/').collect::<PathBuf>());
            if entry.is_dir {
                tokio::fs::create_dir(&path).await?;
                continue;
            }
            let file = tokio::fs::File::create(&path).await?;
            if entry.size > 0 {
                incoming.file = Some((file, entry.size));
                return Ok(None);
            }
        }
        let incoming = self.current.take().expect("aktarım sürüyor");
        Ok(Some(Completed { event_id: incoming.event_id, lamport: incoming.lamport, list: incoming.list }))
    }
}

/// Kabul edilen aktarımı tarihli klasöre taşır, eskileri budar; panoya konacak
/// üst düzey yolları döner.
pub fn accept(root: &Path, event_id: &EventId, list: &FileList) -> anyhow::Result<(PathBuf, Vec<PathBuf>)> {
    let stamp = chrono::Local::now().format("%Y-%m-%d %H.%M.%S").to_string();
    let mut target = root.join(&stamp);
    for attempt in 2.. {
        if !target.exists() {
            break;
        }
        target = root.join(format!("{stamp} ({attempt})"));
    }
    std::fs::rename(partial_dir(root, event_id), &target).context("gelen dosyalar taşınamadı")?;
    prune(root, &target);
    let tops = list.entries.iter().filter(|entry| !entry.path.contains('/')).map(|entry| target.join(&entry.path)).collect();
    Ok((target, tops))
}

pub fn discard(root: &Path, event_id: &EventId) {
    let _ = std::fs::remove_dir_all(partial_dir(root, event_id));
}

/// Kopan bağlantılardan kalan yarım aktarımları siler.
pub fn clean_partials(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(PARTIAL_PREFIX) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Yalnız bizim adlandırdığımız (tarihle başlayan) klasörlere dokunur; en yeni
/// `KEPT_TRANSFERS` tanesi kalır.
fn prune(root: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    let mut ours: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path != keep && path.is_dir() && path.file_name().is_some_and(|name| is_stamp(&name.to_string_lossy())))
        .collect();
    ours.sort();
    let excess = (ours.len() + 1).saturating_sub(KEPT_TRANSFERS);
    for old in ours.into_iter().take(excess) {
        let _ = std::fs::remove_dir_all(old);
    }
}

/// `2026-10-18 13.05.12` ya da `2026-10-18 13.05.12 (2)`.
fn is_stamp(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 19
        && bytes[..19].iter().enumerate().all(|(index, &byte)| match index {
            4 | 7 => byte == b'-',
            10 => byte == b' ',
            13 | 16 => byte == b'.',
            _ => byte.is_ascii_digit(),
        })
}

#[cfg(test)]
mod tests;
