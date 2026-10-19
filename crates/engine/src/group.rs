//! Gruptaki eşlerin bağlantı durumu: her eş için adresler, bağlantı, yeniden
//! deneme zamanı ve bağlantı kopukken yapılan son kopya.

use std::net::SocketAddr;
use std::time::Duration;

use oc_pairing::{Fingerprint, Peer, device_id_of};
use oc_protocol::{DeviceId, PeerInfo};
use oc_transport::Trust;
use tokio::time::Instant;

use crate::Timing;
use crate::dial::Retry;
use crate::events::PeerSummary;
use crate::link::{Link, LinkId};
use crate::transfer::Pending;

pub struct Member {
    pub peer: Peer,
    pub device_id: DeviceId,
    /// Asıl yönde küçük parmak izli taraf arar. Öteki taraf yalnız `fallback_delay`
    /// içinde bağlantı gelmezse arar (gelen bağlantısı güvenlik duvarına takılan eş).
    pub dialer: bool,
    pub addresses: Vec<SocketAddr>,
    pub link: Option<Link>,
    /// Bağlantı asıl yönde mi açıldı; iki bağlantı birden kurulursa iki uç da bunu tutar.
    pub link_preferred: bool,
    pub retry: Retry,
    /// Bağlanma görevi sürüyor.
    pub dialing: bool,
    pub pending: Option<Pending>,
    pub error: Option<String>,
}

impl Member {
    fn new(peer: Peer, own: &Fingerprint, timing: Timing) -> Self {
        let addresses = peer.last_address.iter().filter_map(|address| address.parse().ok()).collect();
        let dialer = own < &peer.fingerprint;
        Self {
            device_id: device_id_of(&peer.fingerprint),
            dialer,
            addresses,
            link: None,
            link_preferred: false,
            retry: Retry::new(timing, if dialer { Duration::ZERO } else { timing.fallback_delay }),
            dialing: false,
            pending: None,
            error: None,
            peer,
        }
    }

    pub fn hex(&self) -> String {
        hex::encode(self.peer.fingerprint)
    }

    /// Bağlantı yoksa ve deneme sürmüyorsa; zamanı `retry` belirler.
    pub fn wants_dial(&self) -> bool {
        self.link.is_none() && !self.dialing
    }

    /// Yeni oturum mevcut bağlantının yerine geçmeli mi? Asıl yöndeki bağlantı
    /// yedek yöndekine bırakılmaz; geri kalan durumlarda yeni oturum eskisinin
    /// yerine geçer (eş yeniden başladıysa eski bağlantı ölüdür).
    pub fn replaces_link(&self, we_dialed: bool) -> bool {
        let preferred = we_dialed == self.dialer;
        self.link.is_none() || preferred || !self.link_preferred
    }

    pub fn prepend_address(&mut self, address: SocketAddr) {
        self.addresses.retain(|existing| *existing != address);
        self.addresses.insert(0, address);
    }

    fn summary(&self) -> PeerSummary {
        let connected = self.link.is_some();
        let address = if connected { self.addresses.first().map(ToString::to_string) } else { self.peer.last_address.clone() };
        PeerSummary {
            name: self.peer.name.clone(),
            fingerprint_prefix: self.hex()[..16].to_owned(),
            address,
            connected,
            link_error: if connected { None } else { self.error.clone() },
        }
    }
}

pub struct Group {
    own: Fingerprint,
    timing: Timing,
    pub members: Vec<Member>,
}

impl Group {
    pub fn new(own: Fingerprint, timing: Timing, peers: Vec<Peer>) -> Self {
        let members = peers.into_iter().map(|peer| Member::new(peer, &own, timing)).collect();
        Self { own, timing, members }
    }

    pub fn add(&mut self, peer: Peer) {
        self.members.push(Member::new(peer, &self.own, self.timing));
    }

    pub fn remove(&mut self, fingerprint: &Fingerprint) -> Option<Member> {
        let index = self.members.iter().position(|member| &member.peer.fingerprint == fingerprint)?;
        Some(self.members.remove(index))
    }

    pub fn by_fingerprint(&mut self, fingerprint: &Fingerprint) -> Option<&mut Member> {
        self.members.iter_mut().find(|member| &member.peer.fingerprint == fingerprint)
    }

    pub fn by_prefix(&self, prefix: &str) -> Option<Fingerprint> {
        let prefix = prefix.trim().to_lowercase();
        let matches: Vec<_> = self.members.iter().filter(|member| !prefix.is_empty() && member.hex().starts_with(&prefix)).collect();
        // Önek birden çok eşe uyuyorsa hiçbiri kaldırılmaz.
        if matches.len() == 1 { Some(matches[0].peer.fingerprint) } else { None }
    }

    pub fn by_link(&mut self, id: LinkId) -> Option<&mut Member> {
        self.members.iter_mut().find(|member| member.link.as_ref().is_some_and(|link| link.id == id))
    }

    /// Bağlantı kabul ederken: gruptaki herhangi bir eş.
    pub fn trust(&self) -> Trust {
        Trust::Pinned(self.members.iter().map(|member| member.peer.fingerprint).collect())
    }

    pub fn next_dial(&self) -> Option<Instant> {
        self.members.iter().filter(|member| member.wants_dial()).map(|member| member.retry.next_dial).min()
    }

    pub fn summaries(&self) -> Vec<PeerSummary> {
        self.members.iter().map(Member::summary).collect()
    }

    /// `to` eşine gönderilecek grup listesi: onun dışındaki bütün eşler.
    pub fn introductions_for(&self, to: &Fingerprint) -> Vec<PeerInfo> {
        self.members
            .iter()
            .filter(|member| &member.peer.fingerprint != to)
            .map(|member| PeerInfo {
                name: member.peer.name.clone(),
                fingerprint: member.peer.fingerprint,
                address: member.peer.last_address.clone(),
            })
            .collect()
    }
}
