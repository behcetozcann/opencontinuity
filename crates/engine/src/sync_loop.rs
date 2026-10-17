//! Eşle bağlantıyı ayakta tutar ve panoyu iki yönde senkronlar.

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context as _, anyhow};
use oc_core::SyncState;
use oc_discovery::Discovery;
use oc_pairing::{Identity, Peer, PeerStore, device_id_of};
use oc_protocol::{MAX_TEXT_BYTES, Message};
use oc_transport::{Session, Trust};
use tokio::net::TcpListener;
use tokio::time::{Instant, interval, sleep_until, timeout};

use crate::events::{Emitter, Event, PeerSummary};
use crate::link::{Link, next_incoming};
use crate::{Command, Context, Flow, Timing, next_found};

pub async fn run(context: &mut Context, mut peer: Peer) -> anyhow::Result<Flow> {
    let port = context.config.port;
    let timing = context.config.timing;
    // Port açılamasa da arayüz kayıtlı eşi göstersin.
    let summary = summary(&peer);
    context.events.update(|snapshot| {
        snapshot.peer = Some(summary);
        snapshot.connected_ip = None;
    });
    let listener = TcpListener::bind(("0.0.0.0", port)).await.with_context(|| {
        format!("port {port} başka bir program tarafından kullanılıyor (açık kalmış bir `oc run` ya da ikinci OpenContinuity olabilir)")
    })?;
    let discovery = if context.config.discovery {
        Some(Discovery::start(&context.identity.name, &context.identity.fingerprint(), port, false)?)
    } else {
        None
    };
    let mut discovered = discovery.as_ref().map(Discovery::browse).transpose()?;

    let mut state = SyncState::new(context.identity.device_id());
    let peer_device_id = device_id_of(&peer.fingerprint);
    let peer_hex = hex::encode(peer.fingerprint);
    let trust = Trust::Pinned(peer.fingerprint);
    // Yalnız küçük parmak izli taraf bağlanır; çift bağlantı yarışı olmaz.
    let dialer = context.identity.fingerprint() < peer.fingerprint;
    let mut addresses: Vec<SocketAddr> = peer.last_address.iter().filter_map(|address| address.parse().ok()).collect();
    let mut link: Option<Link> = None;
    let mut retry = Retry::new(timing);
    let mut heartbeat = interval(timing.heartbeat_interval);

    let role = if dialer { "bağlanan" } else { "bekleyen" };
    context.events.diagnostic(format!("Çalışıyor. Eş: {} ({role} taraf).", peer.name));
    loop {
        tokio::select! {
            command = context.commands.recv() => match command {
                None | Some(Command::Shutdown) => return Ok(Flow::Shutdown),
                Some(Command::Unpair) => {
                    context.store.remove()?;
                    return Ok(Flow::Continue);
                }
                Some(Command::Pause) => context.events.update(|snapshot| snapshot.paused = true),
                Some(Command::Resume) => context.events.update(|snapshot| snapshot.paused = false),
                Some(_) => {}
            },
            accepted = listener.accept() => {
                let (tcp, remote) = accepted?;
                match timeout(timing.handshake_timeout, oc_transport::accept(tcp, &context.identity, trust, port)).await {
                    Ok(Ok(session)) => {
                        let address = SocketAddr::new(remote.ip(), session.peer_listen_port);
                        link = activate(context, &mut peer, &mut addresses, session, address, &state).await?;
                    }
                    Ok(Err(error)) => context.events.diagnostic(format!("Reddedilen bağlantı: {} ({error})", remote.ip())),
                    Err(_) => context.events.diagnostic(format!("Reddedilen bağlantı: {} (zaman aşımı)", remote.ip())),
                }
            }
            Some(found) = next_found(&mut discovered) => {
                if found.fingerprint_hex == peer_hex {
                    let listed: Vec<String> = found.addresses.iter().map(ToString::to_string).collect();
                    context.events.diagnostic(format!("Eş ağda bulundu: {}", listed.join(", ")));
                    for address in found.addresses.into_iter().rev() {
                        prepend_unique(&mut addresses, address);
                    }
                    if dialer && link.is_none() {
                        retry.dial_now();
                    }
                }
            }
            _ = sleep_until(retry.next_dial), if dialer && link.is_none() => {
                match dial(&addresses, &context.identity, trust, port, timing.dial_timeout).await {
                    Ok((session, address)) => {
                        link = activate(context, &mut peer, &mut addresses, session, address, &state).await?;
                    }
                    Err(failures) => {
                        context.events.diagnostic(format!("Bağlanılamadı: {failures}"));
                        retry.failed();
                    }
                }
            }
            text = context.clipboard.local_texts.recv() => {
                let text = text.ok_or_else(|| anyhow!("pano iş parçacığı durdu"))?;
                // Duraklatılmışken saat ilerletilmez; yoksa eşin sonraki kopyaları eski görünürdü.
                if context.events.snapshot().paused {
                    continue;
                }
                if text.len() > MAX_TEXT_BYTES {
                    context.events.emit(Event::Skipped { reason: format!("Kopya çok büyük ({} bayt), gönderilmedi.", text.len()) });
                    continue;
                }
                let Some(event) = state.on_local_text(text) else { continue };
                let (bytes, lamport) = (event.text.len(), event.lamport);
                match link.as_mut() {
                    Some(active) => match active.send(&Message::Clip(event)).await {
                        Ok(()) => context.events.emit(Event::Sent { bytes, lamport }),
                        Err(error) => disconnect(&mut context.events, &mut link, &mut retry, &error.to_string()),
                    },
                    None => context.events.emit(Event::Skipped { reason: "Eş bağlı değil; kopya gönderilmedi.".into() }),
                }
            }
            message = next_incoming(&mut link) => match message {
                Ok(Message::Clip(event)) => {
                    let bytes = event.text.len();
                    if context.events.snapshot().paused {
                        // Uygulanmasa da saat ilerler; yoksa devamdaki ilk yerel kopya eşin
                        // son kopyasıyla aynı lamport'u alıp reddedilebilirdi.
                        state.observe_peer_clock(event.lamport);
                        context.events.emit(Event::Skipped { reason: format!("Duraklatıldı; gelen kopya ({bytes} bayt) uygulanmadı.") });
                    } else if state.on_remote(&peer_device_id, &event) {
                        context.events.emit(Event::Received { bytes, lamport: event.lamport });
                        context.clipboard.writes.send(event.text).map_err(|_| anyhow!("pano iş parçacığı durdu"))?;
                    } else {
                        let reason = format!("Alındı ama uygulanmadı ({bytes} bayt): yerel kopya daha yeni ya da tekrar.");
                        context.events.emit(Event::Skipped { reason });
                    }
                }
                Ok(Message::Clock { lamport }) => {
                    state.observe_peer_clock(lamport);
                    // Bağlantı ancak eşten ilk geçerli mesaj gelince sağlıklı sayılır;
                    // el sıkışmadan hemen sonra kopan bağlantı beklemeyi sıfırlamaz.
                    retry.healthy();
                }
                Ok(Message::Ping { nonce }) => {
                    if let Some(active) = link.as_mut()
                        && let Err(error) = active.send(&Message::Pong { nonce }).await
                    {
                        disconnect(&mut context.events, &mut link, &mut retry, &error.to_string());
                    }
                }
                Ok(_) => {}
                Err(error) => disconnect(&mut context.events, &mut link, &mut retry, &error.to_string()),
            },
            _ = heartbeat.tick() => {
                let Some(active) = link.as_mut() else { continue };
                if active.last_seen.elapsed() > timing.heartbeat_timeout {
                    let reason = format!("eş {} sn yanıt vermedi", timing.heartbeat_timeout.as_secs_f32());
                    disconnect(&mut context.events, &mut link, &mut retry, &reason);
                } else if let Err(error) = active.send(&Message::Ping { nonce: 0 }).await {
                    disconnect(&mut context.events, &mut link, &mut retry, &error.to_string());
                }
            }
        }
    }
}

/// Oturumu başlatır ve saatimizi bildirir; gönderilemezse oturum kurulmamış sayılır.
async fn activate(
    context: &mut Context,
    peer: &mut Peer,
    addresses: &mut Vec<SocketAddr>,
    session: Session,
    address: SocketAddr,
    state: &SyncState,
) -> anyhow::Result<Option<Link>> {
    remember_address(&context.store, peer, addresses, address)?;
    let mut link = Link::start(session, context.config.timing.heartbeat_timeout);
    if let Err(error) = link.send(&Message::Clock { lamport: state.clock() }).await {
        context.events.diagnostic(format!("Saat gönderilemedi, bağlantı kapatıldı: {error}"));
        return Ok(None);
    }
    let summary = summary(peer);
    context.events.update(|snapshot| {
        snapshot.peer = Some(summary);
        snapshot.connected_ip = Some(address.ip().to_string());
    });
    Ok(Some(link))
}

/// Adresleri sırayla dener; hepsi başarısızsa her adresin hatasını döner.
async fn dial(
    addresses: &[SocketAddr],
    identity: &Identity,
    trust: Trust,
    listen_port: u16,
    dial_timeout: Duration,
) -> Result<(Session, SocketAddr), String> {
    if addresses.is_empty() {
        return Err("bilinen adres yok; eş ağda henüz bulunamadı".into());
    }
    let mut failures = Vec::new();
    for address in addresses {
        let attempt = oc_transport::connect(*address, identity, trust, listen_port);
        match timeout(dial_timeout, attempt).await {
            Ok(Ok(session)) => return Ok((session, *address)),
            Ok(Err(error)) => failures.push(format!("{address} ({error:#})")),
            Err(_) => failures.push(format!("{address} (zaman aşımı)")),
        }
    }
    Err(failures.join(", "))
}

fn disconnect(events: &mut Emitter, link: &mut Option<Link>, retry: &mut Retry, reason: &str) {
    if link.take().is_some() {
        events.emit(Event::Disconnected { reason: reason.to_owned() });
        events.update(|snapshot| snapshot.connected_ip = None);
    }
    retry.failed();
}

fn remember_address(store: &PeerStore, peer: &mut Peer, addresses: &mut Vec<SocketAddr>, address: SocketAddr) -> anyhow::Result<()> {
    prepend_unique(addresses, address);
    let text = address.to_string();
    if peer.last_address.as_deref() != Some(text.as_str()) {
        peer.last_address = Some(text);
        store.save(peer)?;
    }
    Ok(())
}

fn prepend_unique(addresses: &mut Vec<SocketAddr>, address: SocketAddr) {
    addresses.retain(|existing| *existing != address);
    addresses.insert(0, address);
}

fn summary(peer: &Peer) -> PeerSummary {
    PeerSummary {
        name: peer.name.clone(),
        fingerprint_prefix: hex::encode(peer.fingerprint)[..16].to_owned(),
        address: peer.last_address.clone(),
    }
}

/// Bağlanan tarafın yeniden deneme zamanlaması: her başarısızlıkta bekleme ikiye katlanır.
struct Retry {
    next_dial: Instant,
    backoff: Duration,
    min: Duration,
    max: Duration,
}

impl Retry {
    fn new(timing: Timing) -> Self {
        Self { next_dial: Instant::now(), backoff: timing.min_backoff, min: timing.min_backoff, max: timing.max_backoff }
    }

    fn failed(&mut self) {
        self.next_dial = Instant::now() + self.backoff;
        self.backoff = (self.backoff * 2).min(self.max);
    }

    fn healthy(&mut self) {
        self.backoff = self.min;
    }

    fn dial_now(&mut self) {
        self.next_dial = Instant::now();
    }
}
