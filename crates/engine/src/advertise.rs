//! Keşif kaynağı: masaüstünde mDNS, iOS'ta kabuğun kendi Bonjour'u (ham
//! multicast soketi iOS'ta Apple'ın ayrı yetkisini ister).

use oc_discovery::{Discovery, Found};
use oc_pairing::Fingerprint;
use tokio::sync::mpsc;

pub enum DiscoveryMode {
    /// Keşif yok (testler); eşler yalnız son bilinen adresle aranır.
    Off,
    Mdns,
    External(ExternalDiscovery),
}

/// Kabuk yayını ve aramayı kendisi yapar: bulduklarını `found`'a yazar,
/// eşleştirme işareti değişince `set_pairing` çağrılır (TXT `pair`).
pub struct ExternalDiscovery {
    pub found: mpsc::Receiver<Found>,
    pub set_pairing: Box<dyn Fn(bool) + Send>,
}

pub(crate) enum Advertiser {
    None,
    Mdns(Discovery),
    External(Box<dyn Fn(bool) + Send>),
}

impl Advertiser {
    pub fn start(
        mode: DiscoveryMode,
        name: &str,
        fingerprint: &Fingerprint,
        port: u16,
    ) -> anyhow::Result<(Self, Option<mpsc::Receiver<Found>>)> {
        Ok(match mode {
            DiscoveryMode::Off => (Self::None, None),
            DiscoveryMode::Mdns => {
                let discovery = Discovery::start(name, fingerprint, port, false)?;
                let found = discovery.browse()?;
                (Self::Mdns(discovery), Some(found))
            }
            DiscoveryMode::External(external) => (Self::External(external.set_pairing), Some(external.found)),
        })
    }

    pub fn set_pairing(&self, on: bool) -> anyhow::Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Mdns(discovery) => discovery.set_pairing(on),
            Self::External(set_pairing) => {
                set_pairing(on);
                Ok(())
            }
        }
    }
}
