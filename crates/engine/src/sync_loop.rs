//! Gruptaki eşlerle bağlantıları ayakta tutar ve panoyu senkronlar. Her eşle
//! ayrı bağlantı (tam ağ); yerel kopya bağlı bütün eşlere gider.

use std::net::SocketAddr;

use anyhow::{Context as _, anyhow};
use oc_core::SyncState;
use oc_discovery::Discovery;
use oc_pairing::{Fingerprint, Peer};
use oc_protocol::Message;
use oc_transport::Session;
use tokio::net::TcpListener;
use tokio::time::{interval, sleep_until, timeout};

use crate::dial::{Dialed, Dialer};
use crate::events::{Emitter, Event};
use crate::group::{Group, Member};
use crate::link::{self, Link, LinkId};
use crate::{Command, Context, Flow, files, next_found, transfer};

pub async fn run(context: &mut Context, peers: Vec<Peer>) -> anyhow::Result<Flow> {
    let port = context.config.port;
    let timing = context.config.timing;
    let own = context.identity.fingerprint();
    let mut group = Group::new(own, timing, peers);
    // Port açılamasa da arayüz kayıtlı eşleri göstersin.
    context.events.update(|snapshot| snapshot.peers = group.summaries());
    let listener = TcpListener::bind(("0.0.0.0", port)).await.with_context(|| {
        format!("port {port} başka bir program tarafından kullanılıyor (açık kalmış bir `oc run` ya da ikinci OpenContinuity olabilir)")
    })?;
    let discovery = if context.config.discovery { Some(Discovery::start(&context.identity.name, &own, port, false)?) } else { None };
    let mut discovered = discovery.as_ref().map(Discovery::browse).transpose()?;

    files::clean_partials(&context.config.files_dir);
    let mut state = SyncState::new(context.identity.device_id());
    let (outlets, mut inlets) = link::channels();
    let (dialer, mut dialed) = Dialer::new(context.identity.clone(), port, timing.dial_timeout);
    let mut next_link_id: LinkId = 0;
    let mut reported: Option<u64> = None;
    let mut heartbeat = interval(timing.heartbeat_interval);

    let names: Vec<&str> = group.members.iter().map(|member| member.peer.name.as_str()).collect();
    context.events.diagnostic(format!("Çalışıyor. Grup: {}.", names.join(", ")));
    loop {
        let next_dial = group.next_dial();
        tokio::select! {
            command = context.commands.recv() => match command {
                None | Some(Command::Shutdown) => return Ok(Flow::Shutdown),
                Some(Command::StartPairing { address }) => return Ok(Flow::Pair { address }),
                Some(Command::Unpair { fingerprint_prefix }) => {
                    if let Some(fingerprint) = group.by_prefix(&fingerprint_prefix) {
                        context.store.remove(&fingerprint)?;
                        group.remove(&fingerprint);
                        if group.members.is_empty() {
                            return Ok(Flow::Continue);
                        }
                    }
                }
                Some(Command::Pause) => context.events.update(|snapshot| snapshot.paused = true),
                Some(Command::Resume) => context.events.update(|snapshot| snapshot.paused = false),
                Some(_) => {}
            },
            accepted = listener.accept() => {
                let (tcp, remote) = accepted?;
                let trust = group.trust();
                match timeout(timing.handshake_timeout, oc_transport::accept(tcp, &context.identity, trust, port)).await {
                    Ok(Ok(session)) => {
                        let address = SocketAddr::new(remote.ip(), session.peer_listen_port);
                        let id = take_id(&mut next_link_id);
                        activate(context, &mut group, &outlets, &state, (session, address), false, id).await?;
                    }
                    Ok(Err(error)) => context.events.diagnostic(format!("Reddedilen bağlantı: {} ({error})", remote.ip())),
                    Err(_) => context.events.diagnostic(format!("Reddedilen bağlantı: {} (zaman aşımı)", remote.ip())),
                }
            }
            Some(found) = next_found(&mut discovered) => {
                if let Some(member) = group.members.iter_mut().find(|member| member.hex() == found.fingerprint_hex) {
                    let listed: Vec<String> = found.addresses.iter().map(ToString::to_string).collect();
                    context.events.diagnostic(format!("{} ağda bulundu: {}", member.peer.name, listed.join(", ")));
                    for address in found.addresses.into_iter().rev() {
                        member.prepend_address(address);
                    }
                    if member.link.is_none() {
                        member.retry.peer_seen();
                    }
                }
            }
            _ = sleep_until(next_dial.unwrap_or_else(tokio::time::Instant::now)), if next_dial.is_some() => {
                let now = tokio::time::Instant::now();
                for member in group.members.iter_mut().filter(|member| member.wants_dial() && member.retry.next_dial <= now) {
                    member.dialing = true;
                    dialer.spawn(member.peer.fingerprint, member.addresses.clone());
                }
            }
            Some(Dialed { fingerprint, result }) = dialed.recv() => {
                let Some(member) = group.by_fingerprint(&fingerprint) else { continue };
                member.dialing = false;
                match result {
                    // Bu arada eş bize bağlandıysa hangisinin kalacağına `activate` karar verir.
                    Ok(connection) => {
                        let id = take_id(&mut next_link_id);
                        activate(context, &mut group, &outlets, &state, connection, true, id).await?;
                    }
                    // Deneme sürerken eş öteki yönden bağlandı.
                    Err(_) if member.link.is_some() => {}
                    Err(failures) => {
                        context.events.diagnostic(format!("{} bağlanılamadı: {failures}", member.peer.name));
                        member.error = Some(failures);
                        member.retry.failed();
                    }
                }
            }
            copy = context.clipboard.local_copies.recv() => {
                let copy = copy.ok_or_else(|| anyhow!("pano iş parçacığı durdu"))?;
                for (fingerprint, reason) in transfer::send_local(context, &mut state, &mut group, copy).await? {
                    if let Some(member) = group.by_fingerprint(&fingerprint) {
                        disconnect(&context.events, member, &reason);
                    }
                }
            }
            Some((id, completion)) = inlets.completions.recv() => {
                if let Some(reason) = transfer::on_completion(context, completion, &mut reported)
                    && let Some(member) = group.by_link(id)
                {
                    disconnect(&context.events, member, &reason);
                }
            }
            Some((id, message)) = inlets.incoming.recv() => {
                // Kapatılmış bağlantının kanalda kalmış mesajı.
                let Some(member) = group.by_link(id) else { continue };
                match message {
                    Ok(Message::Peers { peers }) => {
                        let from = member.peer.fingerprint;
                        introduce(context, &mut group, &from, peers)?;
                    }
                    other => on_message(context, &mut state, member, other)?,
                }
            }
            _ = heartbeat.tick() => {
                for member in &mut group.members {
                    let Some(active) = member.link.as_mut() else { continue };
                    if active.last_seen().elapsed() > timing.heartbeat_timeout {
                        let reason = format!("eş {} sn yanıt vermedi", timing.heartbeat_timeout.as_secs_f32());
                        disconnect(&context.events, member, &reason);
                    } else if let Err(error) = active.post(Message::Ping { nonce: 0 }) {
                        disconnect(&context.events, member, &error.to_string());
                    }
                }
            }
        }
        context.events.update(|snapshot| snapshot.peers = group.summaries());
    }
}

fn take_id(next: &mut LinkId) -> LinkId {
    *next += 1;
    *next
}

fn on_message(context: &mut Context, state: &mut SyncState, member: &mut Member, message: anyhow::Result<Message>) -> anyhow::Result<()> {
    match message {
        Ok(Message::Clip(event)) => transfer::apply_remote(context, state, (&member.device_id, &member.peer.name), event)?,
        Ok(notice @ (Message::ImageStart { .. } | Message::FilesStart { .. } | Message::TransferCancel)) => {
            transfer::on_notice(context, &notice, &member.peer.name);
        }
        Ok(Message::Clock { lamport }) => {
            state.observe_peer_clock(lamport);
            // Bağlantı ancak eşten ilk geçerli mesaj gelince sağlıklı sayılır;
            // el sıkışmadan hemen sonra kopan bağlantı beklemeyi sıfırlamaz.
            member.retry.healthy();
        }
        Ok(Message::Ping { nonce }) => {
            if let Some(active) = member.link.as_mut()
                && let Err(error) = active.post(Message::Pong { nonce })
            {
                disconnect(&context.events, member, &error.to_string());
            }
        }
        Ok(_) => {}
        Err(error) => disconnect(&context.events, member, &error.to_string()),
    }
    Ok(())
}

/// Oturumu başlatır; saatimizi, grup listesini ve kopukken yapılan kopyayı
/// gönderir. Gönderilemezse oturum kurulmamış sayılır. `we_dialed`: oturumu biz
/// mi açtık (asıl yön mü, yedek mi olduğunu belirler).
async fn activate(
    context: &mut Context,
    group: &mut Group,
    outlets: &link::Outlets,
    state: &SyncState,
    (session, address): (Session, SocketAddr),
    we_dialed: bool,
    id: LinkId,
) -> anyhow::Result<()> {
    let fingerprint = session.peer_fingerprint;
    let introductions = group.introductions_for(&fingerprint);
    // Kabul ederken parmak izi grup listesiyle doğrulandı; bu arada kaldırıldıysa bırakılır.
    let Some(member) = group.by_fingerprint(&fingerprint) else { return Ok(()) };
    if !member.replaces_link(we_dialed) {
        context.events.diagnostic(format!("{} ile ikinci bağlantı bırakıldı (asıl yöndeki kalıyor)", member.peer.name));
        return Ok(());
    }
    member.prepend_address(address);
    let text = address.to_string();
    if member.peer.last_address.as_deref() != Some(text.as_str()) {
        member.peer.last_address = Some(text);
        context.store.upsert(&member.peer)?;
    }
    let mut link = Link::start(id, session, context.config.timing.heartbeat_timeout, context.config.files_dir.clone(), outlets);
    let greeting = async {
        link.send(Message::Clock { lamport: state.clock() }).await?;
        link.send(Message::Peers { peers: introductions }).await
    };
    if let Err(error) = greeting.await {
        context.events.diagnostic(format!("{} ile el sıkışma tamamlanamadı, bağlantı kapatıldı: {error}", member.peer.name));
        return Ok(());
    }
    if let Err(error) = transfer::send_pending(context, &mut link, &mut member.pending) {
        context.events.diagnostic(format!("Bekleyen kopya gönderilemedi, bağlantı kapatıldı: {error}"));
        return Ok(());
    }
    context.events.diagnostic(format!("Bağlandı: {} ({address})", member.peer.name));
    member.error = None;
    member.link_preferred = we_dialed == member.dialer;
    member.link = Some(link);
    Ok(())
}

/// Gruptaki bir eşin tanıttığı cihazları ekler ve bağlı diğer eşlere de duyurur.
fn introduce(context: &mut Context, group: &mut Group, from: &Fingerprint, peers: Vec<oc_protocol::PeerInfo>) -> anyhow::Result<()> {
    let own = context.identity.fingerprint();
    let via = group.by_fingerprint(from).map(|member| member.peer.name.clone()).unwrap_or_default();
    let mut added = false;
    for info in peers {
        if info.fingerprint == own {
            continue;
        }
        let peer = Peer { name: info.name, fingerprint: info.fingerprint, last_address: info.address };
        if context.store.add_introduced(&peer)? {
            context.events.emit(Event::PeerAdded { peer: peer.name.clone(), via: via.clone() });
            group.add(peer);
            added = true;
        }
    }
    if !added {
        return Ok(());
    }
    for index in 0..group.members.len() {
        let to = group.members[index].peer.fingerprint;
        if &to == from {
            continue;
        }
        let introductions = group.introductions_for(&to);
        let member = &mut group.members[index];
        if let Some(active) = member.link.as_mut()
            && let Err(error) = active.post(Message::Peers { peers: introductions })
        {
            disconnect(&context.events, member, &error.to_string());
        }
    }
    Ok(())
}

fn disconnect(events: &Emitter, member: &mut Member, reason: &str) {
    if member.link.take().is_some() {
        events.emit(Event::Disconnected { peer: member.peer.name.clone(), reason: reason.to_owned() });
    }
    member.retry.failed();
}
