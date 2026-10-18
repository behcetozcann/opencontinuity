//! Sistem panosuna düz metin, görsel (PNG) ve dosya listesi okuma/yazma,
//! değişiklik algılama.
//!
//! Değişiklik algılama her platformda ucuz bir sayaçla yapılır (macOS
//! `changeCount`, Windows `GetClipboardSequenceNumber`, Linux XFixes
//! sahip değişikliği olayları); içerik yalnızca sayaç değişince okunur.

#[cfg(any(windows, test))]
mod dib;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "linux", test))]
mod uri;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_memory;
#[cfg(target_os = "linux")]
mod x11;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Panodaki içeriğin senkronizasyon açısından anlamı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// Finder/Gezgin'de kopyalanan dosya ve klasörler. Dosya kopyası adını metin,
    /// simgesini görsel olarak da koyar; bu yüzden önce dosyalara bakılır.
    Files(Vec<std::path::PathBuf>),
    /// Panoda metin de görsel de varsa metin seçilir (ofis uygulamaları metnin
    /// görüntüsünü de koyar).
    Text(String),
    /// PNG baytları; platformun başka görsel biçimi okunurken PNG'ye çevrilir.
    Image(Vec<u8>),
    /// Parola yöneticisi vb. tarafından gizli/geçici işaretlenmiş; asla gönderilmez.
    Concealed,
    /// Desteklenen içerik yok (dosya, boş pano).
    Unsupported,
}

pub trait Clipboard {
    /// Pano her değiştiğinde artan sayaç.
    fn sequence(&self) -> u64;
    fn read(&self) -> anyhow::Result<Content>;
    fn write_text(&self, text: &str) -> anyhow::Result<()>;
    fn write_png(&self, png: &[u8]) -> anyhow::Result<()>;
    /// Yapıştırınca Finder/Gezgin bu dosyaları hedefe kopyalar.
    fn write_files(&self, paths: &[std::path::PathBuf]) -> anyhow::Result<()>;
    /// Teşhis için panoyu kimin doldurduğu ve hangi biçimler olduğu; içerik içermez.
    fn describe_source(&self) -> String;
}

/// Sayaç değiştikten sonra okur. Yazan uygulama önce panoyu temizleyip sonra
/// metni koyduğunda, araya giren okuma boş pano görür ve sayaç bir daha artmaz
/// (macOS'ta gözlendi); bu yüzden `Unsupported` bir kez gecikmeli yeniden okunur.
pub fn read_after_change(clipboard: &dyn Clipboard) -> anyhow::Result<Content> {
    let content = clipboard.read()?;
    if content != Content::Unsupported {
        return Ok(content);
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    clipboard.read()
}

/// PNG'nin IEND bloğuyla biten gerçek uzunluğu. Windows pano belleği
/// yuvarlanmış boyutla döner; sondaki dolgu yankı karşılaştırmasını bozmasın.
#[cfg_attr(not(any(windows, target_os = "linux", test)), allow(dead_code))]
fn png_length(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(&PNG_SIGNATURE) {
        return None;
    }
    let mut offset = PNG_SIGNATURE.len();
    loop {
        let header = bytes.get(offset..offset + 8)?;
        let data_length = u32::from_be_bytes(header[..4].try_into().ok()?) as usize;
        let end = offset.checked_add(12)?.checked_add(data_length)?;
        if end > bytes.len() {
            return None;
        }
        if &header[4..] == b"IEND" {
            return Some(end);
        }
        offset = end;
    }
}

pub fn system() -> anyhow::Result<Box<dyn Clipboard>> {
    #[cfg(target_os = "macos")]
    return Ok(Box::new(macos::MacClipboard::new()));
    #[cfg(windows)]
    return Ok(Box::new(windows::WindowsClipboard::new()?));
    #[cfg(target_os = "linux")]
    return Ok(Box::new(x11::X11Clipboard::new()?));
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    anyhow::bail!("bu platformda pano desteği henüz yok");
}

/// Linux Wayland oturumunda X11 panosu yalnız XWayland uygulamalarını görür;
/// arka planda izleme güvenilir değil. Diğer durumlarda `None`.
pub fn session_warning() -> Option<&'static str> {
    let wayland = cfg!(target_os = "linux") && std::env::var("XDG_SESSION_TYPE").is_ok_and(|kind| kind == "wayland");
    wayland.then_some("Wayland oturumu: pano izleme güvenilir değil. Oturum açarken X11 (Cinnamon) oturumunu seçin.")
}

/// macOS 26+ pano erişim davranışı (tanılama için); diğer platformlarda `None`.
pub fn access_behavior() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    return macos::access_behavior();
    #[cfg(not(target_os = "macos"))]
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_length_ignores_trailing_padding() {
        let mut png = PNG_SIGNATURE.to_vec();
        png.extend_from_slice(&[0, 0, 0, 2, b'a', b'b', b'c', b'd', 7, 7, 1, 2, 3, 4]);
        png.extend_from_slice(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 1, 2, 3, 4]);
        let length = png.len();
        png.extend_from_slice(&[0; 13]);
        assert_eq!(png_length(&png), Some(length));
        assert_eq!(png_length(&png[..length - 1]), None, "kesik PNG");
        assert_eq!(png_length(b"GIF89a"), None);
    }
}
