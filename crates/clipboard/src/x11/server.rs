//! CLIPBOARD'u izleyen ve sahibi olduğumuzda içeriği veren iş parçacığı.
//! Sayaç: başka bir uygulama (ya da biz) sahip olunca XFixes olayı gelir.
//! Büyük veri ICCCM INCR ile parça parça verilir: isteyen her parçayı okuyup
//! sildikçe (PropertyNotify/Delete) sıradaki yazılır, boş parça sonu bildirir.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::Event;
use x11rb::protocol::xfixes::{self, ConnectionExt as _, SelectionEventMask};
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, CreateWindowAux, EventMask, PropMode, Property,
    SELECTION_NOTIFY_EVENT, SelectionNotifyEvent, SelectionRequestEvent, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE};

use super::{Atoms, Offer, flush};

/// Yarım bırakılan INCR aktarımı (isteyen kapandı) bu süreden sonra unutulur.
const INCR_ABANDON: Duration = Duration::from_secs(30);

pub struct Server {
    connection: Arc<RustConnection>,
    window: Window,
    pub atoms: Atoms,
    pub sequence: AtomicU64,
    offer: Mutex<Option<Offer>>,
}

struct Incr {
    requestor: Window,
    property: Atom,
    kind: Atom,
    data: Arc<Vec<u8>>,
    offset: usize,
    touched: Instant,
}

impl Server {
    pub fn start() -> anyhow::Result<Arc<Self>> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        let atoms = Atoms::new(&connection)?.reply()?;
        xfixes::query_version(&connection, 5, 0)?.reply().context("X sunucusunda XFixes yok")?;
        let window = connection.generate_id()?;
        connection.create_window(COPY_DEPTH_FROM_PARENT, window, root, 0, 0, 1, 1, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new())?;
        let mask = SelectionEventMask::SET_SELECTION_OWNER | SelectionEventMask::SELECTION_WINDOW_DESTROY | SelectionEventMask::SELECTION_CLIENT_CLOSE;
        connection.xfixes_select_selection_input(window, atoms.CLIPBOARD, mask)?;
        flush(&connection)?;
        let server = Arc::new(Self { connection: Arc::new(connection), window, atoms, sequence: AtomicU64::new(0), offer: Mutex::new(None) });
        let running = server.clone();
        std::thread::Builder::new().name("oc-x11-pano".into()).spawn(move || running.run())?;
        Ok(server)
    }

    /// İçeriği koyar ve CLIPBOARD'un sahibi olur.
    pub fn own(&self, offer: Offer) -> anyhow::Result<()> {
        *self.offer.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(offer);
        self.connection.set_selection_owner(self.window, self.atoms.CLIPBOARD, CURRENT_TIME)?;
        if !self.is_owner() {
            bail!("panonun sahibi olunamadı");
        }
        Ok(())
    }

    fn is_owner(&self) -> bool {
        let owner = self.connection.get_selection_owner(self.atoms.CLIPBOARD).ok().and_then(|cookie| cookie.reply().ok());
        owner.is_some_and(|reply| reply.owner == self.window)
    }

    fn run(&self) {
        let mut transfers: Vec<Incr> = Vec::new();
        loop {
            let event = match self.connection.wait_for_event() {
                Ok(event) => event,
                // X sunucusu kapandı (oturum kapanıyor); sayaç artık değişmez.
                Err(_) => return,
            };
            match event {
                Event::XfixesSelectionNotify(notify) if notify.selection == self.atoms.CLIPBOARD => {
                    self.sequence.fetch_add(1, Ordering::SeqCst);
                }
                Event::SelectionRequest(request) => {
                    if let Some(transfer) = self.serve(&request) {
                        transfers.push(transfer);
                    }
                }
                // Bu olay işlenmeden biz yeniden sahip olmuş olabiliriz; yeni içerik silinmesin.
                Event::SelectionClear(clear) if clear.selection == self.atoms.CLIPBOARD && !self.is_owner() => {
                    *self.offer.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
                }
                Event::PropertyNotify(notify) if notify.state == Property::DELETE => {
                    self.continue_incr(&mut transfers, notify.window, notify.atom);
                }
                _ => {}
            }
            transfers.retain(|transfer| transfer.touched.elapsed() < INCR_ABANDON);
            let _ = self.connection.flush();
        }
    }

    /// İsteğe cevap verir; veri büyükse INCR aktarımını başlatıp döner.
    fn serve(&self, request: &SelectionRequestEvent) -> Option<Incr> {
        // Eski istemciler özellik vermez; hedef adı kullanılır (ICCCM).
        let property = if request.property == NONE { request.target } else { request.property };
        let offer = self.offer.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut incr = None;
        let delivered = match offer.as_ref() {
            Some(offer) if request.selection == self.atoms.CLIPBOARD && request.owner == self.window => {
                if request.target == self.atoms.TARGETS {
                    let mut targets: Vec<Atom> = offer.targets().collect();
                    targets.push(self.atoms.TARGETS);
                    self.connection.change_property32(PropMode::REPLACE, request.requestor, property, AtomEnum::ATOM, &targets).is_ok()
                } else if let Some((_, kind, data)) = offer.find(request.target) {
                    let limit = self.chunk_bytes();
                    if data.len() > limit {
                        incr = self.start_incr(request.requestor, property, *kind, data.clone());
                        incr.is_some()
                    } else {
                        self.connection.change_property8(PropMode::REPLACE, request.requestor, property, *kind, data).is_ok()
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        drop(offer);
        let notify = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: request.time,
            requestor: request.requestor,
            selection: request.selection,
            target: request.target,
            property: if delivered { property } else { NONE },
        };
        let _ = self.connection.send_event(false, request.requestor, EventMask::NO_EVENT, notify);
        incr
    }

    fn chunk_bytes(&self) -> usize {
        (self.connection.maximum_request_bytes() - 1024).min(1 << 20)
    }

    fn start_incr(&self, requestor: Window, property: Atom, kind: Atom, data: Arc<Vec<u8>>) -> Option<Incr> {
        let watch = ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE);
        self.connection.change_window_attributes(requestor, &watch).ok()?;
        let length = u32::try_from(data.len()).ok()?;
        self.connection.change_property32(PropMode::REPLACE, requestor, property, self.atoms.INCR, &[length]).ok()?;
        Some(Incr { requestor, property, kind, data, offset: 0, touched: Instant::now() })
    }

    /// İsteyen önceki parçayı sildi: sıradakini (bittiyse boş parçayı) yazar.
    fn continue_incr(&self, transfers: &mut Vec<Incr>, window: Window, property: Atom) {
        let Some(index) = transfers.iter().position(|transfer| transfer.requestor == window && transfer.property == property) else {
            return;
        };
        let chunk = self.chunk_bytes();
        let transfer = &mut transfers[index];
        let end = (transfer.offset + chunk).min(transfer.data.len());
        let piece = &transfer.data[transfer.offset..end];
        let written = self.connection.change_property8(PropMode::REPLACE, window, property, transfer.kind, piece).is_ok();
        transfer.touched = Instant::now();
        let finished = piece.is_empty() || !written;
        transfer.offset = end;
        if finished {
            let unwatch = ChangeWindowAttributesAux::new().event_mask(EventMask::NO_EVENT);
            let _ = self.connection.change_window_attributes(window, &unwatch);
            transfers.remove(index);
        }
    }
}
