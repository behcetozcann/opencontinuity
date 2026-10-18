//! Dosya yollarının X11 panosundaki biçimleri: `text/uri-list` (RFC 2483) ve
//! Nemo/Caja/Nautilus'un `x-special/gnome-copied-files`'ı ("copy" + URI satırları).

use std::path::{Path, PathBuf};

/// `file://` URI'si; yol baytları ayrılmamış karakterler ve `/` dışında yüzde kodlanır.
pub fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &byte in path_bytes(path).iter() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

pub fn uri_list(paths: &[PathBuf]) -> Vec<u8> {
    paths.iter().map(|path| file_uri(path) + "\r\n").collect::<String>().into_bytes()
}

pub fn gnome_copied_files(paths: &[PathBuf]) -> Vec<u8> {
    let uris: Vec<String> = paths.iter().map(|path| file_uri(path)).collect();
    format!("copy\n{}", uris.join("\n")).into_bytes()
}

/// URI listesindeki yerel dosyalar. `file://` olmayan (bağlantı vb.) bir satır
/// varsa liste dosya kopyası sayılmaz ve `None` döner.
pub fn parse_uri_list(bytes: &[u8]) -> Option<Vec<PathBuf>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let lines = text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#'));
    let paths: Option<Vec<PathBuf>> = lines.map(path_of_uri).collect();
    paths.filter(|paths| !paths.is_empty())
}

/// İlk satır işlem ("copy" ya da "cut"), kalanlar URI.
pub fn parse_gnome_copied_files(bytes: &[u8]) -> Option<Vec<PathBuf>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    if !matches!(lines.next()?.trim(), "copy" | "cut") {
        return None;
    }
    parse_uri_list(lines.collect::<Vec<_>>().join("\n").as_bytes())
}

fn path_of_uri(uri: &str) -> Option<PathBuf> {
    // `file:///yol` ya da `file://localhost/yol`; başka makinedeki dosya kabul edilmez.
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    Some(path_from_bytes(percent_decode(rest)?))
}

fn percent_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    Some(decoded)
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(bytes))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_and_spaces_round_trip() {
        let paths = vec![PathBuf::from("/home/behçet/Masaüstü/rapor 2026 #1.pdf"), PathBuf::from("/tmp/a%b")];
        let list = uri_list(&paths);
        assert!(std::str::from_utf8(&list).unwrap().starts_with("file:///home/beh%C3%A7et/Masa%C3%BCst%C3%BC/rapor%202026%20%231.pdf\r\n"));
        assert_eq!(parse_uri_list(&list), Some(paths.clone()));
        assert_eq!(parse_gnome_copied_files(&gnome_copied_files(&paths)), Some(paths));
    }

    #[test]
    fn non_file_uris_are_not_files() {
        assert_eq!(parse_uri_list(b"https://example.com/\r\n"), None);
        assert_eq!(parse_uri_list(b"file:///a\r\nhttps://example.com/\r\n"), None);
        assert_eq!(parse_uri_list(b"file://baska-makine/a"), None);
        assert_eq!(parse_uri_list(b"# yorum\r\nfile://localhost/a\r\n"), Some(vec![PathBuf::from("/a")]));
        assert_eq!(parse_gnome_copied_files(b"file:///a"), None, "işlem satırı yok");
        assert_eq!(parse_gnome_copied_files(b"cut\nfile:///a"), Some(vec![PathBuf::from("/a")]));
    }
}
