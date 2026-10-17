use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::sel;
use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior, NSPasteboardTypeString};
use objc2_foundation::NSString;

use crate::{Clipboard, Content};

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

    fn has_sensitive_type(&self) -> bool {
        let Some(types) = self.pasteboard.types() else {
            return false;
        };
        types.iter().any(|pasteboard_type| {
            let name = pasteboard_type.to_string();
            SENSITIVE_TYPES.contains(&name.as_str())
        })
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
        Ok(match self.pasteboard.stringForType(string_type) {
            Some(text) => Content::Text(text.to_string()),
            None => Content::NoText,
        })
    }

    fn write_text(&self, text: &str) -> anyhow::Result<()> {
        // SAFETY: bkz. read.
        let string_type = unsafe { NSPasteboardTypeString };
        self.pasteboard.clearContents();
        if !self.pasteboard.setString_forType(&NSString::from_str(text), string_type) {
            anyhow::bail!("NSPasteboard metni kabul etmedi");
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
