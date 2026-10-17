//! Yerel ağda `_opencontinuity._tcp.local.` servisini yayınlar ve arar.
//!
//! Keşif yalnızca adres verir; güven her zaman TLS'teki parmak izine dayanır,
//! TXT kaydındaki parmak izi sadece doğru cihazı seçmek içindir.

use std::net::{IpAddr, SocketAddr};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use oc_pairing::Fingerprint;
use tokio::sync::mpsc;

const SERVICE_TYPE: &str = "_opencontinuity._tcp.local.";

#[derive(Debug, Clone)]
pub struct Found {
    pub name: String,
    pub fingerprint_hex: String,
    /// Cihaz eşleştirme modunda mı (`oc pair` çalışıyor).
    pub pairing: bool,
    pub addresses: Vec<SocketAddr>,
}

pub struct Discovery {
    daemon: ServiceDaemon,
    own_fingerprint_hex: String,
}

impl Discovery {
    pub fn start(name: &str, fingerprint: &Fingerprint, port: u16, pairing: bool) -> anyhow::Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let fingerprint_hex = hex::encode(fingerprint);
        let instance = format!("{name} {}", &fingerprint_hex[..8]);
        let host = format!("oc-{}.local.", &fingerprint_hex[..12]);
        let properties = [("fp", fingerprint_hex.as_str()), ("name", name), ("pair", if pairing { "1" } else { "0" })];
        let service = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, &properties[..])?.enable_addr_auto();
        daemon.register(service)?;
        Ok(Self { daemon, own_fingerprint_hex: fingerprint_hex })
    }

    /// Başka cihazlar çözümlendikçe kanala yazar; kendi yayınımız atlanır.
    pub fn browse(&self) -> anyhow::Result<mpsc::Receiver<Found>> {
        let events = self.daemon.browse(SERVICE_TYPE)?;
        let own = self.own_fingerprint_hex.clone();
        let (sender, receiver) = mpsc::channel(16);
        tokio::spawn(async move {
            while let Ok(event) = events.recv_async().await {
                let ServiceEvent::ServiceResolved(service) = event else { continue };
                let Some(fingerprint_hex) = service.get_property_val_str("fp").map(str::to_owned) else { continue };
                if fingerprint_hex == own {
                    continue;
                }
                let found = Found {
                    name: service.get_property_val_str("name").unwrap_or("?").to_owned(),
                    pairing: service.get_property_val_str("pair") == Some("1"),
                    addresses: usable_addresses(service.get_addresses_v4().into_iter().map(IpAddr::V4), service.port),
                    fingerprint_hex,
                };
                if found.addresses.is_empty() {
                    continue;
                }
                if sender.send(found).await.is_err() {
                    break;
                }
            }
        });
        Ok(receiver)
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

/// Link-local ve loopback dışındaki adresler önce gelir.
fn usable_addresses(addresses: impl Iterator<Item = IpAddr>, port: u16) -> Vec<SocketAddr> {
    let mut sorted: Vec<IpAddr> = addresses.filter(|address| !address.is_loopback()).collect();
    sorted.sort_by_key(|address| matches!(address, IpAddr::V4(v4) if v4.is_link_local()));
    sorted.into_iter().map(|address| SocketAddr::new(address, port)).collect()
}
