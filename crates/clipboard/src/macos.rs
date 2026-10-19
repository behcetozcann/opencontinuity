use std::path::PathBuf;

use anyhow::Context as _;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2::sel;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSPasteboard, NSPasteboardAccessBehavior, NSPasteboardContentsOptions, NSPasteboardTypeFileURL,
    NSPasteboardItem, NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString, NSURL};

use crate::{Clipboard, Content, PNG_SIGNATURE};

/// nspasteboard.org kuralları: parola yöneticileri bu tiplerle işaretler.
const SENSITIVE_TYPES: [&str; 2] = ["org.nspasteboard.ConcealedType", "org.nspasteboard.TransientType"];

const SOURCE_TYPE: &str = "org.nspasteboard.source";

pub struct MacClipboard {
    pasteboard: Retained<NSPasteboard>,
}

impl MacClipboard {
    pub fn new() -> Self {
        Self { pasteboard: NSPasteboard::generalPasteboard() }
    }

    /// Panoyu temizler; yazılacak içerik Evrensel Pano'yla (Handoff) başka Apple
    /// cihazına taşınmaz. Taşınırsa iPhone'daki uygulama onu yeni kopya sanıp geri
    /// gönderiyordu (gözlendi: aynı metin 0,5 sn arayla 10 kez döndü).
    fn clear_for_write(&self) {
        self.pasteboard.prepareForNewContentsWithOptions(NSPasteboardContentsOptions::CurrentHostOnly);
    }

    fn has_sensitive_type(&self) -> bool {
        let Some(types) = self.pasteboard.types() else {
            return false;
        };
        types.iter().any(|pasteboard_type| {
            let name = pasteboard_type.to_string();
            SENSITIVE_TYPES.contains(&name.as_str())
        })
    }

    /// Her kopyalanan öğe ayrı pano öğesidir. Finder dosya referans URL'si
    /// (`file:///.file/id=…`) yazabilir; `filePathURL` onu yola çevirir.
    fn read_files(&self) -> Option<Vec<PathBuf>> {
        // SAFETY: AppKit'in sabit, ömrü süresiz NSString'i.
        let file_url_type = unsafe { NSPasteboardTypeFileURL };
        let paths: Vec<PathBuf> = self
            .pasteboard
            .pasteboardItems()?
            .iter()
            .filter_map(|item| {
                let url = NSURL::URLWithString(&*item.stringForType(file_url_type)?)?;
                Some(PathBuf::from(url.filePathURL()?.path()?.to_string()))
            })
            .collect();
        (!paths.is_empty()).then_some(paths)
    }

    /// Ekran görüntüleri ve tarayıcılar PNG koyar; Önizleme gibi uygulamalar yalnız TIFF.
    fn read_png(&self) -> Option<Vec<u8>> {
        // SAFETY: AppKit'in sabit, ömrü süresiz NSString'leri.
        let (png_type, tiff_type) = unsafe { (NSPasteboardTypePNG, NSPasteboardTypeTIFF) };
        if let Some(png) = self.pasteboard.dataForType(png_type).map(|data| data.to_vec())
            && png.starts_with(&PNG_SIGNATURE)
        {
            return Some(png);
        }
        let tiff = self.pasteboard.dataForType(tiff_type)?;
        let bitmap = NSBitmapImageRep::imageRepWithData(&tiff)?;
        // SAFETY: boş özellik sözlüğü PNG için geçerli.
        let png = unsafe { bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
        Some(png.to_vec())
    }
}

impl Clipboard for MacClipboard {
    fn sequence(&self) -> u64 {
        self.pasteboard.changeCount() as u64
    }

    fn read(&self) -> anyhow::Result<Content> {
        if self.has_sensitive_type() {
            return Ok(Content::Concealed);
        }
        // SAFETY: AppKit'in sabit, ömrü süresiz NSString'i.
        let string_type = unsafe { NSPasteboardTypeString };
        if let Some(paths) = self.read_files() {
            return Ok(Content::Files(paths));
        }
        let text = self.pasteboard.stringForType(string_type).map(|text| text.to_string());
        if let Some(text) = text.as_ref().filter(|text| !text.is_empty()) {
            return Ok(Content::Text(text.clone()));
        }
        Ok(match (self.read_png(), text) {
            (Some(png), _) => Content::Image(png),
            (None, Some(empty)) => Content::Text(empty),
            (None, None) => Content::Unsupported,
        })
    }

    fn write_text(&self, text: &str) -> anyhow::Result<()> {
        // SAFETY: bkz. read.
        let string_type = unsafe { NSPasteboardTypeString };
        self.clear_for_write();
        if !self.pasteboard.setString_forType(&NSString::from_str(text), string_type) {
            anyhow::bail!("NSPasteboard metni kabul etmedi");
        }
        Ok(())
    }

    fn write_png(&self, png: &[u8]) -> anyhow::Result<()> {
        // SAFETY: bkz. read_png.
        let png_type = unsafe { NSPasteboardTypePNG };
        self.clear_for_write();
        if !self.pasteboard.setData_forType(Some(&NSData::with_bytes(png)), png_type) {
            anyhow::bail!("NSPasteboard görseli kabul etmedi");
        }
        Ok(())
    }

    /// Her dosya, URL'si hemen yazılmış ayrı bir pano öğesidir. `NSURL` nesnelerini
    /// doğrudan yazınca ilki dışındakiler tembel veri oluyor, yazan süreç
    /// kapanınca kayboluyordu (gözlendi: 2 öğe yazıldı, 1 okundu).
    fn write_files(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        // SAFETY: bkz. read_files.
        let file_url_type = unsafe { NSPasteboardTypeFileURL };
        let mut items = Vec::new();
        for path in paths {
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
            let address = url.absoluteString().context("dosya adresi oluşturulamadı")?;
            let item = NSPasteboardItem::new();
            if !item.setString_forType(&address, file_url_type) {
                anyhow::bail!("pano öğesi oluşturulamadı");
            }
            items.push(ProtocolObject::from_retained(item));
        }
        self.clear_for_write();
        if !self.pasteboard.writeObjects(&NSArray::from_retained_slice(&items)) {
            anyhow::bail!("NSPasteboard dosyaları kabul etmedi");
        }
        Ok(())
    }

    fn describe_source(&self) -> String {
        let types: Vec<String> = self
            .pasteboard
            .types()
            .map(|types| types.iter().map(|pasteboard_type| pasteboard_type.to_string()).collect())
            .unwrap_or_default();
        // nspasteboard.org: bazı uygulamalar kendi paket kimliğini bu tipe yazar.
        let source = self
            .pasteboard
            .stringForType(&NSString::from_str(SOURCE_TYPE))
            .map(|source| source.to_string())
            .unwrap_or_else(|| "?".to_owned());
        format!("kaynak {source}, tipler [{}]", types.join(", "))
    }
}

pub fn access_behavior() -> Option<&'static str> {
    let pasteboard = NSPasteboard::generalPasteboard();
    // accessBehavior macOS 15.4'ten önce yok; eski sistemde çağırmak çöker.
    if !pasteboard.respondsToSelector(sel!(accessBehavior)) {
        return None;
    }
    Some(match pasteboard.accessBehavior() {
        NSPasteboardAccessBehavior::Default => "default",
        NSPasteboardAccessBehavior::Ask => "ask",
        NSPasteboardAccessBehavior::AlwaysAllow => "always-allow",
        NSPasteboardAccessBehavior::AlwaysDeny => "always-deny",
        _ => "unknown",
    })
}

