//! Win32 pano belleği (HGLOBAL) okuma/yazma ve panoyu açık tutan bekçi.

use std::thread::sleep;
use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};

const OPEN_RETRIES: u32 = 10;

pub fn utf16_until_nul(bytes: &[u8]) -> anyhow::Result<String> {
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let length = units.iter().position(|&unit| unit == 0).unwrap_or(units.len());
    Ok(String::from_utf16(&units[..length])?)
}

/// Pano açıkken çağrılmalı. Biçim yoksa ya da okunamazsa `None`. Boyut
/// yuvarlanmış olabilir; gerçek uzunluğu biçimin kendisi belirler.
pub unsafe fn read_global(format: u32) -> Option<Vec<u8>> {
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
pub unsafe fn set_global(format: u32, bytes: &[u8]) -> anyhow::Result<()> {
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
pub unsafe fn read_dword(format: u32) -> Option<u32> {
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
pub struct OpenGuard;

impl OpenGuard {
    pub fn open() -> anyhow::Result<Self> {
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
