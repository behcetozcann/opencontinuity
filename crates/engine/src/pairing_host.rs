//! Senkron döngüsünün eşleştirme görevini yönetmesi: başlatma, girdileri
//! iletme ve sonucu gruba uygulama.

use oc_transport::Trust;

use crate::Context;
use crate::advertise::Advertiser;
use crate::events::Event;
use crate::group::Group;
use crate::pairing::{Input, Pairing, Update};
use crate::sync_loop::announce_group;

/// Eşleştirme zaten sürüyorsa yenisi başlamaz.
pub(crate) fn start(context: &mut Context, advertiser: &Advertiser, pairing: &mut Option<Pairing>, address: Option<String>) {
    if pairing.is_some() {
        return;
    }
    set_discovery_pairing(context, advertiser, true);
    *pairing = Some(Pairing::start(&context.config, context.identity.clone(), address));
}

pub(crate) fn forward(pairing: &Option<Pairing>, input: Input) {
    if let Some(pairing) = pairing {
        pairing.send(input);
    }
}

/// Dinleyicinin güveni: eşleştirme sürerken yeni cihazın eşleştirme oturumu da kabul edilir.
pub(crate) fn accept_trust(group: &Group, pairing: &Option<Pairing>) -> Trust {
    let group = group.fingerprints();
    if pairing.is_some() { Trust::PinnedOrPairing(group) } else { Trust::Pinned(group) }
}

pub(crate) async fn next_update(pairing: &mut Option<Pairing>) -> Option<Update> {
    match pairing {
        Some(pairing) => pairing.updates.recv().await,
        None => std::future::pending().await,
    }
}

pub(crate) fn apply(
    context: &mut Context,
    group: &mut Group,
    advertiser: &Advertiser,
    pairing: &mut Option<Pairing>,
    update: Update,
) -> anyhow::Result<()> {
    let peer = match update {
        Update::Stage(stage) => {
            context.events.update(|snapshot| snapshot.pairing = Some(stage));
            return Ok(());
        }
        Update::Paired(peer) => peer,
        Update::Failed(reason) => {
            finish(context, advertiser, pairing);
            context.events.emit(Event::PairingFailed { reason });
            return Ok(());
        }
        Update::Cancelled => {
            finish(context, advertiser, pairing);
            context.events.diagnostic("Eşleştirme kullanıcı tarafından iptal edildi");
            return Ok(());
        }
    };
    finish(context, advertiser, pairing);
    context.store.upsert(&peer)?;
    context.events.emit(Event::Paired { peer: peer.name.clone() });
    let fingerprint = peer.fingerprint;
    if group.by_fingerprint(&fingerprint).is_none() {
        group.add(peer);
    }
    // Yeni cihaz bağlı diğer eşlere tanıtılır; ona da grup listesi bağlanınca gider.
    announce_group(context, group, &fingerprint);
    Ok(())
}

fn finish(context: &mut Context, advertiser: &Advertiser, pairing: &mut Option<Pairing>) {
    *pairing = None;
    set_discovery_pairing(context, advertiser, false);
    context.events.update(|snapshot| snapshot.pairing = None);
}

fn set_discovery_pairing(context: &Context, advertiser: &Advertiser, on: bool) {
    if let Err(error) = advertiser.set_pairing(on) {
        context.events.diagnostic(format!("mDNS yayını güncellenemedi: {error:#}"));
    }
}
