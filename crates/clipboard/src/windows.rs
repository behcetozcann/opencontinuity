use std::thread::sleep;
use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
    IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::core::w;

use crate::{Clipboard, Content};

const CF_UNICODETEXT: u32 = 13;
const OPEN_RETRIES: u32 = 10;

pub struct WindowsClipboard {
    exclude_format: u32,
    history_format: u32,
    cloud_format: u32,
}

impl WindowsClipboard {
    pub fn new() -> anyhow::Result<Self> {
        // SAFETY: sabit, null ile biten geniş dizgeler.
        let (exclude_format, history_format, cloud_format) = unsafe {
            (
                RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
                RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")),
                RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")),
            )
        };
        if exclude_format == 0 || history_format == 0 || cloud_format == 0 {
            anyhow::bail!("pano biçimleri kaydedilemedi");
        }
        Ok(Self { exclude_format, history_format, cloud_format })
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
        let _guard = OpenGuard::open()?;
        // SAFETY: pano bu iş parçacığında açık; tutamaçlar kilitli okunuyor.
        unsafe {
            if self.is_sensitive() {
                return Ok(Content::Concealed);
            }
            if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
                return Ok(Content::NoText);
            }
            let handle = GetClipboardData(CF_UNICODETEXT)?;
            let global = HGLOBAL(handle.0);
            let pointer = GlobalLock(global) as *const u16;
            if pointer.is_null() {
                anyhow::bail!("pano verisi kilitlenemedi");
            }
            let max_units = GlobalSize(global) / 2;
            let units = std::slice::from_raw_parts(pointer, max_units);
            let length = units.iter().position(|&unit| unit == 0).unwrap_or(max_units);
            let text = String::from_utf16(&units[..length]);
            let _ = GlobalUnlock(global);
            Ok(Content::Text(text?))
        }
    }

    fn write_text(&self, text: &str) -> anyhow::Result<()> {
        let mut units: Vec<u16> = text.encode_utf16().collect();
        units.push(0);
        let _guard = OpenGuard::open()?;
        // SAFETY: bellek units.len() kadar ayrılır; SetClipboardData başarılı
        // olursa sahiplik sisteme geçer.
        unsafe {
            EmptyClipboard()?;
            let global = GlobalAlloc(GMEM_MOVEABLE, units.len() * 2)?;
            let pointer = GlobalLock(global) as *mut u16;
            if pointer.is_null() {
                anyhow::bail!("pano belleği kilitlenemedi");
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), pointer, units.len());
            let _ = GlobalUnlock(global);
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(global.0)))?;
        }
        Ok(())
    }
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
