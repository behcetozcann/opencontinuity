//! Panodaki içeriği ister: `ConvertSelection` → sahip veriyi penceremizin
//! özelliğine yazar → `SelectionNotify`. Büyük veri INCR ile parça parça gelir.

use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, Property, Window, WindowClass};
use x11rb::rust_connection::RustConnection;
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE};

use super::{Atoms, flush};

/// Sahip bu sürede cevap vermezse hedef yok sayılır (donmuş uygulama).
const REPLY_TIMEOUT: Duration = Duration::from_secs(1);
/// INCR'de iki parça arası en uzun bekleme.
const CHUNK_TIMEOUT: Duration = Duration::from_secs(3);
/// En büyük görselden (32 MiB) biraz fazlası; aşan aktarım bırakılır.
const MAX_READ_BYTES: usize = 40 * 1024 * 1024;

pub struct Reader {
    connection: RustConnection,
    window: Window,
    atoms: Atoms,
}

impl Reader {
    pub fn connect() -> anyhow::Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let atoms = Atoms::new(&connection)?.reply()?;
        let window = connection.generate_id()?;
        // INCR parçaları için özellik değişikliklerini dinler.
        let aux = CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE);
        connection.create_window(COPY_DEPTH_FROM_PARENT, window, root, 0, 0, 1, 1, 0, WindowClass::INPUT_OUTPUT, 0, &aux)?;
        flush(&connection)?;
        Ok(Self { connection, window, atoms })
    }

    /// Sahibin sunduğu biçimler; pano boşsa boş liste.
    pub fn targets(&self) -> anyhow::Result<Vec<Atom>> {
        let owner = self.connection.get_selection_owner(self.atoms.CLIPBOARD)?.reply()?.owner;
        if owner == NONE {
            return Ok(Vec::new());
        }
        let Some(bytes) = self.convert(self.atoms.TARGETS)? else { return Ok(Vec::new()) };
        Ok(bytes.as_chunks::<4>().0.iter().map(|atom| u32::from_ne_bytes(*atom)).collect())
    }

    /// Hedefin baytları; sahip vermediyse ya da cevap gelmediyse `None`.
    pub fn convert(&self, target: Atom) -> anyhow::Result<Option<Vec<u8>>> {
        let property = self.atoms.OC_DATA;
        self.connection.delete_property(self.window, property)?;
        self.connection.convert_selection(self.window, self.atoms.CLIPBOARD, target, property, CURRENT_TIME)?;
        flush(&self.connection)?;
        let deadline = Instant::now() + REPLY_TIMEOUT;
        let notified = self.wait(deadline, |event| match event {
            Event::SelectionNotify(notify) if notify.requestor == self.window && notify.target == target => Some(notify.property),
            _ => None,
        })?;
        let Some(delivered) = notified.filter(|delivered| *delivered != NONE) else { return Ok(None) };
        let reply = self.connection.get_property(true, self.window, delivered, AtomEnum::ANY, 0, u32::MAX / 4)?.reply()?;
        flush(&self.connection)?;
        if reply.type_ == self.atoms.INCR {
            return self.receive_incr(delivered).map(Some);
        }
        Ok(Some(reply.value))
    }

    /// Özellik silindi (yukarıda); sahip her parçayı yazdıkça okuyup siler, boş parça sonudur.
    fn receive_incr(&self, property: Atom) -> anyhow::Result<Vec<u8>> {
        let mut data = Vec::new();
        loop {
            let deadline = Instant::now() + CHUNK_TIMEOUT;
            let arrived = self.wait(deadline, |event| match event {
                Event::PropertyNotify(notify) if notify.window == self.window && notify.atom == property && notify.state == Property::NEW_VALUE => Some(()),
                _ => None,
            })?;
            arrived.context("pano parçası zamanında gelmedi")?;
            let reply = self.connection.get_property(true, self.window, property, AtomEnum::ANY, 0, u32::MAX / 4)?.reply()?;
            flush(&self.connection)?;
            if reply.value.is_empty() {
                return Ok(data);
            }
            data.extend_from_slice(&reply.value);
            if data.len() > MAX_READ_BYTES {
                bail!("pano içeriği {MAX_READ_BYTES} baytı aşıyor");
            }
        }
    }

    /// Koşula uyan olay gelene kadar bekler; süre dolarsa `None`.
    fn wait<T>(&self, deadline: Instant, mut matches: impl FnMut(&Event) -> Option<T>) -> anyhow::Result<Option<T>> {
        loop {
            while let Some(event) = self.connection.poll_for_event()? {
                if let Some(found) = matches(&event) {
                    return Ok(Some(found));
                }
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Sahip pencerenin uygulama sınıfı ve biçim adları; içerik okunmaz.
    pub fn describe(&self) -> anyhow::Result<String> {
        let owner = self.connection.get_selection_owner(self.atoms.CLIPBOARD)?.reply()?.owner;
        if owner == NONE {
            return Ok("sahip yok".into());
        }
        let class = self.connection.get_property(false, owner, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 64)?.reply()?;
        let class = String::from_utf8_lossy(&class.value).replace('\0', " ").trim().to_owned();
        let names: Vec<String> = self
            .targets()?
            .into_iter()
            .filter_map(|atom| self.connection.get_atom_name(atom).ok()?.reply().ok())
            .map(|reply| String::from_utf8_lossy(&reply.name).into_owned())
            .collect();
        let class = if class.is_empty() { "sınıfsız pencere".to_owned() } else { class };
        Ok(format!("{class} (pencere {owner:#x}); biçimler: {}", names.join(", ")))
    }
}
