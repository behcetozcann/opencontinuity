//! Linux (X11) panosu. İki bağlantı kullanılır:
//! - sunucu (`server.rs`): gizli pencereyle CLIPBOARD'u izler (XFixes; sayaç)
//!   ve biz yazdığımızda içeriği isteyen uygulamalara verir — X11'de pano
//!   verisi sahibinin sürecinde durur, sahip kapanınca kaybolur;
//! - okuyucu (`reader.rs`): başka uygulamanın (ya da bizim) içeriğini ister.
//!
//! Görsel baytları çözülmeden taşınır (`image/png`); yankı karşılaştırması
//! (`core::content_hash`) aynı baytları bekler.

mod reader;
mod server;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, AtomEnum};
use x11rb::rust_connection::RustConnection;

use crate::{Clipboard, Content, png_length, uri};

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        CLIPBOARD,
        TARGETS,
        MULTIPLE,
        INCR,
        UTF8_STRING,
        TEXT,
        TEXT_PLAIN_UTF8: b"text/plain;charset=utf-8",
        IMAGE_PNG: b"image/png",
        URI_LIST: b"text/uri-list",
        GNOME_FILES: b"x-special/gnome-copied-files",
        PASSWORD_HINT: b"x-kde-passwordManagerHint",
        OC_DATA: b"OC_CLIPBOARD_DATA",
    }
}

/// Bizim panoya koyduğumuz içerik: her hedef (biçim) için tür ve baytlar.
pub struct Offer {
    entries: Vec<(Atom, Atom, Arc<Vec<u8>>)>,
}

impl Offer {
    fn find(&self, target: Atom) -> Option<&(Atom, Atom, Arc<Vec<u8>>)> {
        self.entries.iter().find(|(entry_target, ..)| *entry_target == target)
    }

    fn targets(&self) -> impl Iterator<Item = Atom> + '_ {
        self.entries.iter().map(|(target, ..)| *target)
    }
}

pub struct X11Clipboard {
    server: Arc<server::Server>,
    reader: Mutex<reader::Reader>,
}

impl X11Clipboard {
    pub fn new() -> anyhow::Result<Self> {
        let server = server::Server::start().context(no_display_hint())?;
        let reader = reader::Reader::connect().context(no_display_hint())?;
        Ok(Self { server, reader: Mutex::new(reader) })
    }

    fn offer(&self, entries: Vec<(Atom, Atom, Vec<u8>)>) -> anyhow::Result<()> {
        let entries = entries.into_iter().map(|(target, kind, bytes)| (target, kind, Arc::new(bytes))).collect();
        self.server.own(Offer { entries })
    }

    fn atoms(&self) -> &Atoms {
        &self.server.atoms
    }
}

fn no_display_hint() -> String {
    match std::env::var("XDG_SESSION_TYPE") {
        Ok(kind) if kind == "wayland" => "X11 panosuna bağlanılamadı (Wayland oturumu; oturum açarken X11 seçin)".into(),
        _ => "X11 panosuna bağlanılamadı (DISPLAY tanımlı değil ya da X sunucusu yok)".into(),
    }
}

impl Clipboard for X11Clipboard {
    fn sequence(&self) -> u64 {
        self.server.sequence.load(Ordering::SeqCst)
    }

    fn read(&self) -> anyhow::Result<Content> {
        let atoms = *self.atoms();
        let reader = self.reader.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let targets = reader.targets()?;
        if targets.is_empty() {
            return Ok(Content::Unsupported);
        }
        if targets.contains(&atoms.PASSWORD_HINT) && reader.convert(atoms.PASSWORD_HINT)?.is_some_and(|hint| hint.trim_ascii() == b"secret") {
            return Ok(Content::Concealed);
        }
        let files = [(atoms.GNOME_FILES, uri::parse_gnome_copied_files as fn(&[u8]) -> _), (atoms.URI_LIST, uri::parse_uri_list)];
        for (target, parse) in files {
            if targets.contains(&target)
                && let Some(paths) = reader.convert(target)?.and_then(|bytes| parse(&bytes))
            {
                return Ok(Content::Files(paths));
            }
        }
        let mut text = None;
        for target in [atoms.UTF8_STRING, atoms.TEXT_PLAIN_UTF8, AtomEnum::STRING.into()] {
            if targets.contains(&target)
                && let Some(bytes) = reader.convert(target)?
            {
                text = Some(decode_text(bytes, target == Atom::from(AtomEnum::STRING)));
                break;
            }
        }
        if let Some(text) = text.as_ref().filter(|text| !text.is_empty()) {
            return Ok(Content::Text(text.clone()));
        }
        if targets.contains(&atoms.IMAGE_PNG)
            && let Some(png) = reader.convert(atoms.IMAGE_PNG)?
            && let Some(length) = png_length(&png)
        {
            return Ok(Content::Image(png[..length].to_vec()));
        }
        Ok(text.map_or(Content::Unsupported, Content::Text))
    }

    fn write_text(&self, text: &str) -> anyhow::Result<()> {
        let atoms = *self.atoms();
        let bytes = text.as_bytes().to_vec();
        // Eski uygulamalar STRING ister; Latin-1 dışı karakterde de UTF-8 vermek yaygın uygulamadır (xclip).
        let string = Atom::from(AtomEnum::STRING);
        self.offer(vec![
            (atoms.UTF8_STRING, atoms.UTF8_STRING, bytes.clone()),
            (atoms.TEXT_PLAIN_UTF8, atoms.TEXT_PLAIN_UTF8, bytes.clone()),
            (atoms.TEXT, atoms.UTF8_STRING, bytes.clone()),
            (string, string, bytes),
        ])
    }

    fn write_png(&self, png: &[u8]) -> anyhow::Result<()> {
        let atoms = *self.atoms();
        self.offer(vec![(atoms.IMAGE_PNG, atoms.IMAGE_PNG, png.to_vec())])
    }

    /// Nemo/Caja/Nautilus `x-special/gnome-copied-files`, diğerleri `text/uri-list` okur.
    fn write_files(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        let atoms = *self.atoms();
        let names = paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join("\n").into_bytes();
        self.offer(vec![
            (atoms.GNOME_FILES, atoms.GNOME_FILES, uri::gnome_copied_files(paths)),
            (atoms.URI_LIST, atoms.URI_LIST, uri::uri_list(paths)),
            (atoms.UTF8_STRING, atoms.UTF8_STRING, names),
        ])
    }

    fn describe_source(&self) -> String {
        let reader = self.reader.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        reader.describe().unwrap_or_else(|error| format!("bilinmiyor ({error})"))
    }
}

/// STRING Latin-1'dir; ama birçok uygulama oraya da UTF-8 koyar.
fn decode_text(bytes: Vec<u8>, latin1: bool) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) if latin1 => error.into_bytes().iter().map(|&byte| char::from(byte)).collect(),
        Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
    }
}

/// Bağlantı koptuysa (X sunucusu kapandı) hata.
fn flush(connection: &RustConnection) -> anyhow::Result<()> {
    connection.flush().context("X11 bağlantısı koptu")
}
