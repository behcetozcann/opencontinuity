use std::path::PathBuf;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::DataExchange::{
    EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW, GetClipboardOwner,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, RegisterClipboardFormatW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::{PWSTR, w};

use crate::windows_memory::{OpenGuard, read_dword, read_global, set_global, utf16_until_nul};
use crate::{Clipboard, Content, dib, png_length};

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
/// DROPFILES başlığı: pFiles (u32) + POINT (2 × i32) + fNC + fWide (BOOL).
const DROPFILES_BYTES: u32 = 20;
const DROPEFFECT_COPY: u32 = 1;

pub struct WindowsClipboard {
    /// Tarayıcılar, Office ve ekran alıntısı aracı bu adla PNG koyar.
    png_format: u32,
    /// Yapıştırmanın taşıma değil kopyalama olduğunu Gezgin'e bildirir.
    drop_effect_format: u32,
    exclude_format: u32,
    history_format: u32,
    cloud_format: u32,
}

impl WindowsClipboard {
    pub fn new() -> anyhow::Result<Self> {
        // SAFETY: sabit, null ile biten geniş dizgeler.
        let (png_format, drop_effect_format, exclude_format, history_format, cloud_format) = unsafe {
            (
                RegisterClipboardFormatW(w!("PNG")),
                RegisterClipboardFormatW(w!("Preferred DropEffect")),
                RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
                RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")),
                RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")),
            )
        };
        if [png_format, drop_effect_format, exclude_format, history_format, cloud_format].contains(&0) {
            anyhow::bail!("pano biçimleri kaydedilemedi");
        }
        Ok(Self { png_format, drop_effect_format, exclude_format, history_format, cloud_format })
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
                if let Some(paths) = read_dropped_files() {
                    return Ok(Content::Files(paths));
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

    fn write_files(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        let mut bytes = Vec::new();
        for field in [DROPFILES_BYTES, 0, 0, 0, 1] {
            // pFiles, pt.x, pt.y, fNC, fWide (UTF-16 yollar)
            bytes.extend_from_slice(&field.to_le_bytes());
        }
        for path in paths {
            bytes.extend(path.as_os_str().to_string_lossy().encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
        }
        bytes.extend_from_slice(&[0, 0]);
        let _guard = OpenGuard::open()?;
        // SAFETY: pano bu iş parçacığında açık.
        unsafe {
            EmptyClipboard()?;
            set_global(CF_HDROP, &bytes)?;
            set_global(self.drop_effect_format, &DROPEFFECT_COPY.to_le_bytes())
        }
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

/// Pano açıkken çağrılmalı. Gezgin'de kopyalanan dosyaların yolları.
unsafe fn read_dropped_files() -> Option<Vec<PathBuf>> {
    unsafe {
        IsClipboardFormatAvailable(CF_HDROP).ok()?;
        let drop = HDROP(GetClipboardData(CF_HDROP).ok()?.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        let paths: Vec<PathBuf> = (0..count)
            .filter_map(|index| {
                let length = DragQueryFileW(drop, index, None) as usize;
                let mut buffer = vec![0u16; length + 1];
                let copied = DragQueryFileW(drop, index, Some(&mut buffer)) as usize;
                (copied > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..copied])))
            })
            .collect();
        (!paths.is_empty()).then_some(paths)
    }
}
