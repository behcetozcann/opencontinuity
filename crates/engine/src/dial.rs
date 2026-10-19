//! Eşe bağlanma denemeleri. Her deneme ayrı görevde çalışır; yanıt vermeyen
//! bir eşin zaman aşımı diğer eşlerin bağlantısını ve pano olaylarını bekletmez.

use std::net::SocketAddr;
use std::time::Duration;

use oc_pairing::{Fingerprint, Identity};
use oc_transport::{Session, Trust};
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout};

use crate::Timing;

pub struct Dialed {
    pub fingerprint: Fingerprint,
    /// Başarısızsa her adresin hatası.
    pub result: Result<(Session, SocketAddr), String>,
}

pub struct Dialer {
    identity: Identity,
    listen_port: u16,
    dial_timeout: Duration,
    results: mpsc::UnboundedSender<Dialed>,
}

impl Dialer {
    pub fn new(identity: Identity, listen_port: u16, dial_timeout: Duration) -> (Self, mpsc::UnboundedReceiver<Dialed>) {
        let (results, receiver) = mpsc::unbounded_channel();
        (Self { identity, listen_port, dial_timeout, results }, receiver)
    }

    /// Adresleri sırayla dener; sonucu kanala yazar.
    pub fn spawn(&self, fingerprint: Fingerprint, addresses: Vec<SocketAddr>) {
        let (identity, listen_port, dial_timeout) = (self.identity.clone(), self.listen_port, self.dial_timeout);
        let results = self.results.clone();
        tokio::spawn(async move {
            let result = dial(&addresses, &identity, fingerprint, listen_port, dial_timeout).await;
            let _ = results.send(Dialed { fingerprint, result });
        });
    }
}

async fn dial(
    addresses: &[SocketAddr],
    identity: &Identity,
    fingerprint: Fingerprint,
    listen_port: u16,
    dial_timeout: Duration,
) -> Result<(Session, SocketAddr), String> {
    if addresses.is_empty() {
        return Err("bilinen adres yok; eş ağda henüz bulunamadı".into());
    }
    let mut failures = Vec::new();
    for address in addresses {
        let attempt = oc_transport::connect(*address, identity, Trust::Pinned(vec![fingerprint]), listen_port);
        match timeout(dial_timeout, attempt).await {
            Ok(Ok(session)) => return Ok((session, *address)),
            Ok(Err(error)) => failures.push(format!("{address} ({error:#})")),
            Err(_) => failures.push(format!("{address} (zaman aşımı)")),
        }
    }
    Err(failures.join(", "))
}

/// Yeniden deneme zamanlaması: her başarısızlıkta bekleme ikiye katlanır.
/// `floor` hiçbir denemenin daha erken yapılmadığı süredir: asıl yönde arayan
/// tarafta sıfır, yedek arayan tarafta `fallback_delay` (asıl yöne önce fırsat).
pub struct Retry {
    pub next_dial: Instant,
    backoff: Duration,
    min: Duration,
    max: Duration,
    floor: Duration,
}

impl Retry {
    pub fn new(timing: Timing, floor: Duration) -> Self {
        let next_dial = Instant::now() + floor;
        Self { next_dial, backoff: timing.min_backoff, min: timing.min_backoff, max: timing.max_backoff, floor }
    }

    pub fn failed(&mut self) {
        self.next_dial = Instant::now() + self.backoff.max(self.floor);
        self.backoff = (self.backoff * 2).min(self.max);
    }

    pub fn healthy(&mut self) {
        self.backoff = self.min;
    }

    /// Eş ağda görüldü: deneme öne alınır (en erken `floor` sonra), ertelenmez.
    pub fn peer_seen(&mut self) {
        self.next_dial = self.next_dial.min(Instant::now() + self.floor);
    }
}
