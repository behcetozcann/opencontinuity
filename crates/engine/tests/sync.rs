mod common;

use std::time::Duration;

use common::{Node, code_of, pair, paired_and_connected};
use oc_engine::{Command, Event, PairingStage};
use oc_pairing::{Identity, Peer, PeerStore};
use oc_protocol::Message;
use oc_transport::Trust;
use tokio::net::TcpListener;

#[tokio::test]
async fn pairs_and_syncs_both_ways_without_loop() {
    let (mac, windows) = paired_and_connected().await;

    mac.copy.send("merhaba".into()).await.unwrap();
    assert_eq!(windows.next_written().await, "merhaba");
    // Gerçek panoda yazılan metin izleyiciye yerel kopya olarak geri döner.
    windows.copy.send("merhaba".into()).await.unwrap();
    mac.assert_nothing_written(Duration::from_millis(400)).await;

    let text = "Çağrı ışığı 👋\r\nikinci satır";
    windows.copy.send(text.into()).await.unwrap();
    assert_eq!(mac.next_written().await, text);
}

#[tokio::test]
async fn pause_blocks_sending_and_applying() {
    let (mut mac, windows) = paired_and_connected().await;
    mac.handle.send(Command::Pause);
    mac.wait_state(|snapshot| snapshot.paused).await;

    mac.copy.send("gizli not".into()).await.unwrap();
    windows.assert_nothing_written(Duration::from_millis(400)).await;
    windows.copy.send("windows kopyası".into()).await.unwrap();
    mac.wait_for(|event| matches!(event, Event::Skipped { .. })).await;
    mac.assert_nothing_written(Duration::from_millis(200)).await;

    mac.handle.send(Command::Resume);
    mac.wait_state(|snapshot| !snapshot.paused).await;
    mac.copy.send("devam".into()).await.unwrap();
    assert_eq!(windows.next_written().await, "devam");
}

#[tokio::test]
async fn unpaired_device_rejects_old_peer_and_can_pair_again() {
    let (mut mac, mut windows) = paired_and_connected().await;
    mac.handle.send(Command::Unpair);
    mac.wait_state(|snapshot| snapshot.peer.is_none()).await;
    windows.wait_for(|event| matches!(event, Event::Disconnected { .. })).await;
    assert!(PeerStore::new(mac.dir.path()).load().unwrap().is_none());

    // Mac eşleştirme beklerken eski eş (hâlâ senkron modunda) bağlanamaz.
    mac.handle.send(Command::StartPairing { address: None });
    mac.wait_state(|snapshot| matches!(snapshot.pairing, Some(PairingStage::Searching { .. }))).await;
    let old_peer = Identity::load_or_create(windows.dir.path(), "windows").unwrap();
    let mac_identity = Identity::load_or_create(mac.dir.path(), "mac").unwrap();
    let attempt =
        oc_transport::connect(("127.0.0.1", mac.port), &old_peer, Trust::Pinned(mac_identity.fingerprint()), windows.port).await;
    assert!(attempt.is_err(), "senkron modundaki eski eş eşleştirme dinleyicisine bağlanmamalı");
    mac.handle.send(Command::CancelPairing);
    mac.wait_state(|snapshot| snapshot.pairing.is_none()).await;

    windows.handle.send(Command::Unpair);
    windows.wait_state(|snapshot| snapshot.peer.is_none()).await;
    pair(&mut mac, &mut windows).await;
    mac.wait_connected().await;
    mac.copy.send("yeniden".into()).await.unwrap();
    assert_eq!(windows.next_written().await, "yeniden");
}

#[tokio::test]
async fn rejecting_code_saves_nothing() {
    let (mut mac, mut windows) = (Node::start("mac"), Node::start("windows"));
    mac.handle.send(Command::StartPairing { address: None });
    mac.wait_state(|snapshot| matches!(snapshot.pairing, Some(PairingStage::Searching { .. }))).await;
    windows.handle.send(Command::StartPairing { address: Some(format!("127.0.0.1:{}", mac.port)) });
    code_of(&mut mac).await;
    code_of(&mut windows).await;
    windows.handle.send(Command::ConfirmCode(true));
    mac.handle.send(Command::ConfirmCode(false));
    mac.wait_for(|event| matches!(event, Event::PairingFailed { .. })).await;
    windows.wait_for(|event| matches!(event, Event::PairingFailed { .. })).await;
    assert!(PeerStore::new(mac.dir.path()).load().unwrap().is_none());
    assert!(PeerStore::new(windows.dir.path()).load().unwrap().is_none());
}

/// Eş saatini gönderip susarsa heartbeat zaman aşımıyla bağlantı kapatılır.
#[tokio::test]
async fn silent_peer_is_dropped_by_heartbeat() {
    let mac_dir = tempfile::tempdir().unwrap();
    let silent_dir = tempfile::tempdir().unwrap();
    let mac_identity = Identity::load_or_create(mac_dir.path(), "mac").unwrap();
    let silent = Identity::load_or_create(silent_dir.path(), "sessiz").unwrap();
    let silent_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let silent_port = silent_listener.local_addr().unwrap().port();
    PeerStore::new(mac_dir.path())
        .save(&Peer {
            name: "sessiz".into(),
            fingerprint: silent.fingerprint(),
            last_address: Some(format!("127.0.0.1:{silent_port}")),
        })
        .unwrap();
    let mut mac = Node::start_in("mac", mac_dir);
    // Eş bilgisi yayınlandığında motor portu açmış olur.
    mac.wait_state(|snapshot| snapshot.peer.is_some()).await;
    let trust = Trust::Pinned(mac_identity.fingerprint());

    let mut session = if mac_identity.fingerprint() < silent.fingerprint() {
        let (tcp, _) = silent_listener.accept().await.unwrap();
        oc_transport::accept(tcp, &silent, trust, silent_port).await.unwrap()
    } else {
        oc_transport::connect(("127.0.0.1", mac.port), &silent, trust, silent_port).await.unwrap()
    };
    session.send(&Message::Clock { lamport: 0 }).await.unwrap();
    mac.wait_connected().await;
    let event = mac.wait_for(|event| matches!(event, Event::Disconnected { .. })).await;
    assert!(matches!(event, Event::Disconnected { reason } if reason.contains("yanıt vermedi")));
    drop(session);
}
