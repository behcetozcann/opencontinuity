use std::thread::sleep;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW,
    GetClipboardOwner, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::{PWSTR, w};

use crate::{Clipboard, Content, dib, png_length};

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const OPEN_RETRIES: u32 = 10;

pub struct WindowsClipboard {
    /// Tarayıcılar, Office ve ekran alıntısı aracı bu adla PNG koyar.
    png_format: u32,
    exclude_format: u32,
    history_format: u32,
    cloud_format: u32,
}

impl WindowsClipboard {
    pub fn new() -> anyhow::Result<Self> {
        // SAFETY: sabit, null ile biten geniş dizgeler.
        let (png_format, exclude_format, history_format, cloud_format) = unsafe {
            (
                RegisterClipboardFormatW(w!("PNG")),
                RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
                RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")),
                RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")),
            )
        };
        if png_format == 0 || exclude_format == 0 || history_format == 0 || cloud_format == 0 {
            anyhow::bail!("pano biçimleri kaydedilemedi");
        }
        Ok(Self { png_format, exclude_format, history_format, cloud_format })
    }

    /// Pano açıkken çağrılmalı.
    unsafe fn is_sensitive(&self) -> bool {
        unsafe {
            IsClipboardFormatAvailable(self.exclude_format).is_ok()
                || read_dword(self.history_format) == Some(0)
                || read_dword(self.cloud_format) == Some(0)
        }
    }
}

impl Clipboard for WindowsClipboard {
    fn sequence(&self) -> u64 {
        // SAFETY: parametresiz, yan etkisiz Win32 çağrısı.
        u64::from(unsafe { GetClipboardSequenceNumber() })
    }

    fn read(&self) -> anyhow::Result<Content> {
        // Pano yalnız baytlar kopyalanırken açık tutulur; DIB→PNG dönüşümü kapandıktan sonra.
        let (text, image) = {
            let _guard = OpenGuard::open()?;
            // SAFETY: pano bu iş parçacığında açık.
            unsafe {
                if self.is_sensitive() {
                    return Ok(Content::Concealed);
                }
                let text = read_global(CF_UNICODETEXT).map(|bytes| utf16_until_nul(&bytes)).transpose()?;
                if let Some(text) = text.as_ref().filter(|text| !text.is_empty()) {
                    return Ok(Content::Text(text.clone()));
                }
                let png = read_global(self.png_format).and_then(|bytes| png_length(&bytes).map(|length| bytes[..length].to_vec()));
                let image = match png {
                    Some(png) => Some(Ok(png)),
                    None => read_global(CF_DIB).map(Err),
                };
                (text, image)
            }
        };
        Ok(match (image, text) {
            (Some(Ok(png)), _) => Content::Image(png),
            (Some(Err(dib_bytes)), _) => Content::Image(dib::dib_to_png(&dib_bytes)?),
            (None, Some(empty)) => Content::Text(empty),
            (None, None) => Content::Unsupported,
        })
    }

    fn write_text(&self, text: &str) -> anyhow::Result<()> {
        let bytes: Vec<u8> = text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
        let _guard = OpenGuard::open()?;
        // SAFETY: pano bu iş parçacığında açık.
        unsafe {
            EmptyClipboard()?;
            set_global(CF_UNICODETEXT, &bytes)
        }
    }

    /// PNG'yi bilmeyen uygulamalar (Paint vb.) için CF_DIB de konur.
    fn write_png(&self, png: &[u8]) -> anyhow::Result<()> {
        let dib = dib::png_to_dib(png);
        let _guard = OpenGuard::open()?;
        // SAFETY: pano bu iş parçacığında açık.
        unsafe {
            EmptyClipboard()?;
            set_global(self.png_format, png)?;
            if let Ok(dib) = dib {
                set_global(CF_DIB, &dib)?;
            }
        }
        Ok(())
    }

    fn describe_source(&self) -> String {
        // SAFETY: salt okunur sorgular; biçim listesi pano açıkken okunur.
        let owner = unsafe { owner_process() }.unwrap_or_else(|| "?".to_owned());
        let formats = match OpenGuard::open() {
            Ok(_guard) => unsafe { list_formats() },
            Err(_) => "pano kilitli".to_owned(),
        };
        format!("sahip {owner}, biçimler [{formats}]")
    }
}

unsafe fn owner_process() -> Option<String> {
    unsafe {
        let window = GetClipboardOwner().ok()?;
        let mut process_id = 0u32;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
        let mut buffer = [0u16; 260];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size);
        let _ = CloseHandle(process);
        result.ok()?;
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        Some(path.rsplit('\\').next().unwrap_or(&path).to_owned())
    }
}

/// Pano açıkken çağrılmalı.
unsafe fn list_formats() -> String {
    let mut names = Vec::new();
    let mut format = 0;
    loop {
        format = unsafe { EnumClipboardFormats(format) };
        if format == 0 {
            break;
        }
        names.push(format_name(format));
    }
    names.join(", ")
}

fn format_name(format: u32) -> String {
    let standard = match format {
        1 => "CF_TEXT",
        2 => "CF_BITMAP",
        7 => "CF_OEMTEXT",
        8 => "CF_DIB",
        13 => "CF_UNICODETEXT",
        15 => "CF_HDROP",
        16 => "CF_LOCALE",
        17 => "CF_DIBV5",
        _ => "",
    };
    if !standard.is_empty() {
        return standard.to_owned();
    }
    let mut buffer = [0u16; 128];
    // SAFETY: tampon boyutu fonksiyona dilim olarak verilir.
    let length = unsafe { GetClipboardFormatNameW(format, &mut buffer) };
    if length > 0 { String::from_utf16_lossy(&buffer[..length as usize]) } else { format!("#{format}") }
}

fn utf16_until_nul(bytes: &[u8]) -> anyhow::Result<String> {
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let length = units.iter().position(|&unit| unit == 0).unwrap_or(units.len());
    Ok(String::from_utf16(&units[..length])?)
}

/// Pano açıkken çağrılmalı. Biçim yoksa ya da okunamazsa `None`. Boyut
/// yuvarlanmış olabilir; gerçek uzunluğu biçimin kendisi belirler.
unsafe fn read_global(format: u32) -> Option<Vec<u8>> {
    unsafe {
        IsClipboardFormatAvailable(format).ok()?;
        let global = HGLOBAL(GetClipboardData(format).ok()?.0);
        let pointer = GlobalLock(global) as *const u8;
        if pointer.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(pointer, GlobalSize(global)).to_vec();
        let _ = GlobalUnlock(global);
        Some(bytes)
    }
}

/// Pano açık ve boşaltılmışken çağrılmalı; başarılıysa belleğin sahipliği sisteme geçer.
unsafe fn set_global(format: u32, bytes: &[u8]) -> anyhow::Result<()> {
    unsafe {
        let global = GlobalAlloc(GMEM_MOVEABLE, bytes.len())?;
        let pointer = GlobalLock(global) as *mut u8;
        if pointer.is_null() {
            anyhow::bail!("pano belleği kilitlenemedi");
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
        let _ = GlobalUnlock(global);
        SetClipboardData(format, Some(HANDLE(global.0)))?;
    }
    Ok(())
}

/// Pano açıkken çağrılmalı. Biçim yoksa `None`.
unsafe fn read_dword(format: u32) -> Option<u32> {
    unsafe {
        IsClipboardFormatAvailable(format).ok()?;
        let global = HGLOBAL(GetClipboardData(format).ok()?.0);
        if GlobalSize(global) < 4 {
            return None;
        }
        let pointer = GlobalLock(global) as *const u32;
        if pointer.is_null() {
            return None;
        }
        let value = pointer.read_unaligned();
        let _ = GlobalUnlock(global);
        Some(value)
    }
}

/// Pano başka uygulamada açıksa kısa aralıklarla yeniden dener; düşünce kapatır.
struct OpenGuard;

impl OpenGuard {
    fn open() -> anyhow::Result<Self> {
        for _ in 0..OPEN_RETRIES {
            // SAFETY: sahipsiz açma; CloseClipboard Drop'ta çağrılır.
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(Self);
            }
            sleep(Duration::from_millis(20));
        }
        anyhow::bail!("pano başka bir uygulama tarafından kilitli")
    }
}

impl Drop for OpenGuard {
    fn drop(&mut self) {
        // SAFETY: open() başarılı olduysa pano bu iş parçacığında açık.
        let _ = unsafe { CloseClipboard() };
    }
}
