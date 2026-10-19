//! Eşleştirme: bağlantı → iki ekranda aynı 6 haneli kod → iki uç da onaylayınca
//! eş kaydedilir. Biri reddederse hiçbir şey kaydedilmez. Senkron döngüsünün
//! yanında ayrı görev olarak çalışır; gruptaki bağlantılar bu sırada sürer.

use std::net::SocketAddr;

use anyhow::{anyhow, bail};
use oc_discovery::Found;
use oc_pairing::{Identity, Peer, sas_code};
use oc_protocol::Message;
use oc_transport::{Session, Trust};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{Instant, sleep_until, timeout};

use crate::events::PairingStage;
use crate::link::{self, Link};
use crate::{EngineConfig, config};

/// Senkron döngüsünden göreve.
pub(crate) enum Input {
    Confirm(bool),
    Cancel,
    /// Dinleyicinin kabul ettiği eşleştirme oturumu.
    Accepted(Box<Session>, SocketAddr),
    /// mDNS'te eşleştirme modundaki cihaz.
    Found(Found),
}

/// Görevden senkron döngüsüne.
pub(crate) enum Update {
    Stage(PairingStage),
    Paired(Peer),
    Failed(String),
    Cancelled,
}

pub(crate) struct Pairing {
    inputs: UnboundedSender<Input>,
    pub updates: UnboundedReceiver<Update>,
}

impl Pairing {
    /// `address` yoksa aynı ağda eşleştirme modundaki cihaz aranır ve beklenir.
    pub fn start(config: &EngineConfig, identity: Identity, address: Option<String>) -> Self {
        let (inputs, input_receiver) = mpsc::unbounded_channel();
        let (update_sender, updates) = mpsc::unbounded_channel();
        let settings = Settings::from(config);
        tokio::spawn(async move {
            let update = match pair(&settings, &identity, address, input_receiver, &update_sender).await {
                Ok(Some(peer)) => Update::Paired(peer),
                Ok(None) => Update::Cancelled,
                Err(error) => Update::Failed(format!("{error:#}")),
            };
            let _ = update_sender.send(update);
        });
        Self { inputs, updates }
    }

    pub fn send(&self, input: Input) {
        let _ = self.inputs.send(input);
    }
}

/// Görevin ihtiyaç duyduğu ayarlar (motorun ayarlarının kopyası).
struct Settings {
    port: u16,
    timing: crate::Timing,
    files_dir: std::path::PathBuf,
}

impl From<&EngineConfig> for Settings {
    fn from(config: &EngineConfig) -> Self {
        Self { port: config.port, timing: config.timing, files_dir: config.files_dir.clone() }
    }
}

async fn pair(
    settings: &Settings,
    identity: &Identity,
    address: Option<String>,
    mut inputs: UnboundedReceiver<Input>,
    updates: &UnboundedSender<Update>,
) -> anyhow::Result<Option<Peer>> {
    let local_ip = config::local_ip().map(|ip| ip.to_string());
    let _ = updates.send(Update::Stage(PairingStage::Searching { local_ip }));
    let Some((session, peer_address)) = find_session(settings, identity, address, &mut inputs).await? else {
        return Ok(None);
    };

    let code = sas_code(&identity.fingerprint(), &session.peer_fingerprint, &session.exporter);
    let peer = Peer {
        name: session.peer_name.clone(),
        fingerprint: session.peer_fingerprint,
        last_address: Some(peer_address.to_string()),
    };
    let (outlets, mut inlets) = link::channels();
    let mut link = Link::start(0, session, settings.timing.heartbeat_timeout, settings.files_dir.clone(), &outlets);
    let _ = updates.send(Update::Stage(PairingStage::Confirm { peer: peer.name.clone(), code }));

    let mut own_accepted = false;
    let mut peer_accepted = false;
    while !(own_accepted && peer_accepted) {
        tokio::select! {
            input = inputs.recv() => match input {
                Some(Input::Confirm(accepted)) if !own_accepted => {
                    link.send(Message::PairDecision { accepted }).await?;
                    if !accepted {
                        bail!("eşleştirme reddedildi; hiçbir şey kaydedilmedi");
                    }
                    own_accepted = true;
                    if !peer_accepted {
                        let _ = updates.send(Update::Stage(PairingStage::WaitingPeer { peer: peer.name.clone() }));
                    }
                }
                None | Some(Input::Cancel) => {
                    let _ = link.send(Message::PairDecision { accepted: false }).await;
                    return Ok(None);
                }
                // Kod onayı sürerken gelen ikinci aday bırakılır.
                Some(_) => {}
            },
            message = inlets.incoming.recv() => match message.map_or_else(|| Err(anyhow!("okuma görevi bitti")), |(_, message)| message) {
                Ok(Message::PairDecision { accepted: true }) => peer_accepted = true,
                Ok(Message::PairDecision { accepted: false }) => bail!("karşı cihaz reddetti; hiçbir şey kaydedilmedi"),
                Ok(_) => bail!("eş beklenmeyen mesaj gönderdi"),
                Err(error) => bail!("eşleştirme bağlantısı koptu: {error}"),
            },
        }
    }
    Ok(Some(peer))
}

/// Adres verilmişse oraya bağlanır. Verilmemişse dinleyicinin kabul ettiği
/// oturumu ya da mDNS'te bulunan eşleştirme modundaki cihazı bekler: parmak izi
/// küçük olan taraf bağlanır; öteki `fallback_delay` içinde bağlantı gelmezse
/// kendisi arar (gelen bağlantısı güvenlik duvarına takılıyorsa).
async fn find_session(
    settings: &Settings,
    identity: &Identity,
    address: Option<String>,
    inputs: &mut UnboundedReceiver<Input>,
) -> anyhow::Result<Option<(Session, SocketAddr)>> {
    if let Some(address) = address {
        let address = with_default_port(&address, settings.port)?;
        let attempt = timeout(settings.timing.handshake_timeout, oc_transport::connect(address, identity, Trust::Pairing, settings.port));
        return tokio::select! {
            session = attempt => Ok(Some((session.map_err(|_| anyhow!("{address} yanıt vermedi"))??, address))),
            () = next_cancel(inputs) => Ok(None),
        };
    }
    let own_hex = hex::encode(identity.fingerprint());
    let mut fallback: Option<(Instant, Vec<SocketAddr>)> = None;
    loop {
        let fallback_at = fallback.as_ref().map(|(at, _)| *at);
        tokio::select! {
            input = inputs.recv() => match input {
                None | Some(Input::Cancel) => return Ok(None),
                Some(Input::Accepted(session, address)) => return Ok(Some((*session, address))),
                Some(Input::Found(found)) if found.pairing && found.fingerprint_hex < own_hex => {
                    if fallback.is_none() {
                        fallback = Some((Instant::now() + settings.timing.fallback_delay, found.addresses));
                    }
                }
                Some(Input::Found(found)) if found.pairing => {
                    if let Some(connected) = dial(settings, identity, &found.addresses).await {
                        return Ok(Some(connected));
                    }
                }
                Some(_) => {}
            },
            _ = sleep_until(fallback_at.unwrap_or_else(Instant::now)), if fallback_at.is_some() => {
                let Some((at, addresses)) = fallback.as_mut() else { continue };
                if let Some(connected) = dial(settings, identity, addresses).await {
                    return Ok(Some(connected));
                }
                *at = Instant::now() + settings.timing.fallback_delay;
            }
        }
    }
}

/// Adresle bağlanırken yalnız iptal dinlenir; diğer girdiler atlanır.
async fn next_cancel(inputs: &mut UnboundedReceiver<Input>) {
    while let Some(input) = inputs.recv().await {
        if matches!(input, Input::Cancel) {
            return;
        }
    }
}

async fn dial(settings: &Settings, identity: &Identity, addresses: &[SocketAddr]) -> Option<(Session, SocketAddr)> {
    for address in addresses {
        let attempt = oc_transport::connect(*address, identity, Trust::Pairing, settings.port);
        if let Ok(Ok(session)) = timeout(settings.timing.dial_timeout, attempt).await {
            return Some((session, *address));
        }
    }
    None
}

fn with_default_port(address: &str, port: u16) -> anyhow::Result<SocketAddr> {
    let address = address.trim();
    if let Ok(address) = address.parse::<SocketAddr>() {
        return Ok(address);
    }
    let ip = address.parse().map_err(|_| anyhow!("'{address}' geçerli bir IP adresi değil"))?;
    Ok(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_gets_default_port() {
        assert_eq!(with_default_port(" 192.168.1.82 ", 47800).unwrap(), "192.168.1.82:47800".parse().unwrap());
        assert_eq!(with_default_port("10.0.0.2:5000", 47800).unwrap(), "10.0.0.2:5000".parse().unwrap());
        assert!(with_default_port("windows-pc", 47800).is_err());
    }
}
