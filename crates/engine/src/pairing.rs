//! İlk eşleştirme: bağlantı → iki ekranda aynı 6 haneli kod → iki uç da
//! onaylayınca eş kaydedilir. Biri reddederse hiçbir şey kaydedilmez.

use std::net::SocketAddr;

use anyhow::{anyhow, bail};
use oc_discovery::Discovery;
use oc_pairing::{Identity, Peer, sas_code};
use oc_protocol::Message;
use oc_transport::{Session, Trust};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{Instant, sleep_until, timeout};

use crate::events::{Emitter, Event, PairingStage};
use crate::link::{self, Link};
use crate::{ClipboardPort, Command, Context, EngineConfig, Flow, config, next_found};

/// Hata olursa `PairingFailed` yayınlanır ve boşta durumuna dönülür.
pub async fn run(context: &mut Context, address: Option<String>) -> anyhow::Result<Flow> {
    let result = pair(context, address).await;
    context.events.update(|snapshot| snapshot.pairing = None);
    match result {
        Ok(flow) => Ok(flow),
        Err(error) => {
            context.events.emit(Event::PairingFailed { reason: format!("{error:#}") });
            Ok(Flow::Continue)
        }
    }
}

async fn pair(context: &mut Context, address: Option<String>) -> anyhow::Result<Flow> {
    let local_ip = config::local_ip().map(|ip| ip.to_string());
    context.events.update(|snapshot| snapshot.pairing = Some(PairingStage::Searching { local_ip }));
    let (session, peer_address) = tokio::select! {
        connected = connect(&context.config, &context.identity, address, context.events.sender()) => connected?,
        flow = wait_for_cancel(&mut context.commands, &mut context.clipboard, &mut context.events) => return Ok(flow),
    };

    let code = sas_code(&context.identity.fingerprint(), &session.peer_fingerprint, &session.exporter);
    let peer = Peer {
        name: session.peer_name.clone(),
        fingerprint: session.peer_fingerprint,
        last_address: Some(peer_address.to_string()),
    };
    let (outlets, mut inlets) = link::channels();
    let mut link = Link::start(0, session, context.config.timing.heartbeat_timeout, context.config.files_dir.clone(), &outlets);
    let name = peer.name.clone();
    context.events.update(|snapshot| snapshot.pairing = Some(PairingStage::Confirm { peer: name, code }));

    let mut own_accepted = false;
    let mut peer_accepted = false;
    while !(own_accepted && peer_accepted) {
        tokio::select! {
            command = context.commands.recv() => match command {
                Some(Command::ConfirmCode(accepted)) if !own_accepted => {
                    link.send(Message::PairDecision { accepted }).await?;
                    if !accepted {
                        bail!("eşleştirme reddedildi; hiçbir şey kaydedilmedi");
                    }
                    own_accepted = true;
                    if !peer_accepted {
                        let name = peer.name.clone();
                        context.events.update(|snapshot| snapshot.pairing = Some(PairingStage::WaitingPeer { peer: name }));
                    }
                }
                Some(Command::CancelPairing) => {
                    let _ = link.send(Message::PairDecision { accepted: false }).await;
                    return Ok(Flow::Continue);
                }
                None | Some(Command::Shutdown) => return Ok(Flow::Shutdown),
                Some(Command::Pause) => context.events.update(|snapshot| snapshot.paused = true),
                Some(Command::Resume) => context.events.update(|snapshot| snapshot.paused = false),
                Some(_) => {}
            },
            message = inlets.incoming.recv() => match message.map_or_else(|| Err(anyhow!("okuma görevi bitti")), |(_, message)| message) {
                Ok(Message::PairDecision { accepted: true }) => peer_accepted = true,
                Ok(Message::PairDecision { accepted: false }) => bail!("karşı cihaz reddetti; hiçbir şey kaydedilmedi"),
                Ok(_) => bail!("eş beklenmeyen mesaj gönderdi"),
                Err(error) => bail!("eşleştirme bağlantısı koptu: {error}"),
            },
            copy = context.clipboard.local_copies.recv() => {
                copy.ok_or_else(|| anyhow!("pano iş parçacığı durdu"))?;
            }
        }
    }
    context.store.upsert(&peer)?;
    context.events.emit(Event::Paired { peer: peer.name });
    Ok(Flow::Continue)
}

/// Adres verilmişse oraya bağlanır. Verilmemişse dinler ve mDNS ile yayın
/// yapar; eşleştirme modundaki cihaz bulunursa parmak izi küçük olan taraf
/// bağlanır, diğeri bağlantıyı kabul eder.
async fn connect(
    config: &EngineConfig,
    identity: &Identity,
    address: Option<String>,
    diagnostics: UnboundedSender<Event>,
) -> anyhow::Result<(Session, SocketAddr)> {
    let diagnostic = |message: String| {
        let _ = diagnostics.send(Event::Diagnostic { message });
    };
    if let Some(address) = address {
        let address = with_default_port(&address, config.port)?;
        let session = timeout(config.timing.handshake_timeout, oc_transport::connect(address, identity, Trust::Pairing, config.port))
            .await
            .map_err(|_| anyhow!("{address} yanıt vermedi"))??;
        return Ok((session, address));
    }
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    let discovery =
        if config.discovery { Some(Discovery::start(&identity.name, &identity.fingerprint(), config.port, true)?) } else { None };
    let mut discovered = discovery.as_ref().map(Discovery::browse).transpose()?;
    let own_hex = hex::encode(identity.fingerprint());
    // Asıl yönde küçük parmak izli taraf arar; öteki `fallback_delay` içinde
    // bağlantı gelmezse kendisi arar (gelen bağlantısı güvenlik duvarına takılıyorsa).
    let mut fallback: Option<(Instant, Vec<SocketAddr>)> = None;
    loop {
        let fallback_at = fallback.as_ref().map(|(at, _)| *at);
        tokio::select! {
            accepted = listener.accept() => {
                let (tcp, remote) = accepted?;
                match timeout(config.timing.handshake_timeout, oc_transport::accept(tcp, identity, Trust::Pairing, config.port)).await {
                    Ok(Ok(session)) => {
                        let address = SocketAddr::new(remote.ip(), session.peer_listen_port);
                        return Ok((session, address));
                    }
                    Ok(Err(error)) => diagnostic(format!("Reddedilen eşleştirme bağlantısı: {} ({error})", remote.ip())),
                    Err(_) => diagnostic(format!("Reddedilen eşleştirme bağlantısı: {} (zaman aşımı)", remote.ip())),
                }
            }
            Some(found) = next_found(&mut discovered) => {
                if !found.pairing {
                    continue;
                }
                if found.fingerprint_hex < own_hex {
                    if fallback.is_none() {
                        fallback = Some((Instant::now() + config.timing.fallback_delay, found.addresses));
                    }
                    continue;
                }
                if let Some(connected) = dial_pairing(config, identity, &found.addresses).await {
                    return Ok(connected);
                }
            }
            _ = sleep_until(fallback_at.unwrap_or_else(Instant::now)), if fallback_at.is_some() => {
                let Some((at, addresses)) = fallback.as_mut() else { continue };
                if let Some(connected) = dial_pairing(config, identity, addresses).await {
                    return Ok(connected);
                }
                *at = Instant::now() + config.timing.fallback_delay;
            }
        }
    }
}

async fn dial_pairing(config: &EngineConfig, identity: &Identity, addresses: &[SocketAddr]) -> Option<(Session, SocketAddr)> {
    for address in addresses {
        let attempt = oc_transport::connect(*address, identity, Trust::Pairing, config.port);
        if let Ok(Ok(session)) = timeout(config.timing.dial_timeout, attempt).await {
            return Some((session, *address));
        }
    }
    None
}

/// Bağlantı kurulana kadar iptal ve kapanış komutlarını dinler.
async fn wait_for_cancel(
    commands: &mut UnboundedReceiver<Command>,
    clipboard: &mut ClipboardPort,
    events: &mut Emitter,
) -> Flow {
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(Command::CancelPairing) => return Flow::Continue,
                None | Some(Command::Shutdown) => return Flow::Shutdown,
                Some(Command::Pause) => events.update(|snapshot| snapshot.paused = true),
                Some(Command::Resume) => events.update(|snapshot| snapshot.paused = false),
                Some(_) => {}
            },
            copy = clipboard.local_copies.recv() => {
                if copy.is_none() {
                    return Flow::Shutdown;
                }
            }
        }
    }
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
