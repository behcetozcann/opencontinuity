//! Sistem panosuna düz metin okuma/yazma ve değişiklik algılama.
//!
//! Değişiklik algılama her iki platformda da ucuz bir sayaçla yapılır
//! (macOS `changeCount`, Windows `GetClipboardSequenceNumber`); içerik
//! yalnızca sayaç değişince okunur.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// Panodaki içeriğin senkronizasyon açısından anlamı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text(String),
    /// Parola yöneticisi vb. tarafından gizli/geçici işaretlenmiş; asla gönderilmez.
    Concealed,
    /// Panoda düz metin yok (görsel, dosya, boş pano).
    NoText,
}

pub trait Clipboard {
    /// Pano her değiştiğinde artan sayaç.
    fn sequence(&self) -> u64;
    fn read(&self) -> anyhow::Result<Content>;
    fn write_text(&self, text: &str) -> anyhow::Result<()>;
}

/// Sayaç değiştikten sonra okur. Yazan uygulama önce panoyu temizleyip sonra
/// metni koyduğunda, araya giren okuma boş pano görür ve sayaç bir daha artmaz
/// (macOS'ta gözlendi); bu yüzden `NoText` bir kez gecikmeli yeniden okunur.
pub fn read_after_change(clipboard: &dyn Clipboard) -> anyhow::Result<Content> {
    let content = clipboard.read()?;
    if content != Content::NoText {
        return Ok(content);
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    clipboard.read()
}

pub fn system() -> anyhow::Result<Box<dyn Clipboard>> {
    #[cfg(target_os = "macos")]
    return Ok(Box::new(macos::MacClipboard::new()));
    #[cfg(windows)]
    return Ok(Box::new(windows::WindowsClipboard::new()?));
    #[cfg(not(any(target_os = "macos", windows)))]
    anyhow::bail!("bu platformda pano desteği henüz yok");
}

/// macOS 26+ pano erişim davranışı (tanılama için); diğer platformlarda `None`.
pub fn access_behavior() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    return macos::access_behavior();
    #[cfg(not(target_os = "macos"))]
    None
}
