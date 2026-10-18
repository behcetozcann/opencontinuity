mod common;

use std::time::Duration;

use common::{Node, code_of, pair, paired_and_connected, text};
use oc_engine::{Command, ContentKind, Event, PairingStage, Payload};
use oc_pairing::{Identity, Peer, PeerStore};
use oc_protocol::Message;
use oc_transport::Trust;
use tokio::net::TcpListener;

#[tokio::test]
async fn pairs_and_syncs_both_ways_without_loop() {
    let (mac, windows) = paired_and_connected().await;

    mac.copy.send(text("merhaba")).await.unwrap();
    assert_eq!(windows.next_written().await, text("merhaba"));
    // Gerçek panoda yazılan metin izleyiciye yerel kopya olarak geri döner.
    windows.copy.send(text("merhaba")).await.unwrap();
    mac.assert_nothing_written(Duration::from_millis(400)).await;

    let multiline = text("Çağrı ışığı 👋\r\nikinci satır");
    windows.copy.send(multiline.clone()).await.unwrap();
    assert_eq!(mac.next_written().await, multiline);
}

/// Çerçeve sınırından büyük görsel parçalanarak gider; yankısı geri dönmez.
#[tokio::test]
async fn large_image_syncs_in_chunks_without_echo() {
    let (mut mac, mut windows) = paired_and_connected().await;
    let mut png = oc_protocol::PNG_SIGNATURE.to_vec();
    png.extend((0..oc_protocol::MAX_FRAME_BYTES + 300_000).map(|index| (index % 253) as u8));
    let image = Payload::Png(png);

    mac.copy.send(image.clone()).await.unwrap();
    assert_eq!(windows.next_written().await, image);
    // Arayüz "gönderiliyor/alınıyor" gösterebilsin: başlangıç olayı tamamlanmadan önce gelir.
    mac.wait_for(|event| matches!(event, Event::Sending { content: ContentKind::Image, .. })).await;
    let sent = mac.wait_for(|event| matches!(event, Event::Sent { .. })).await;
    assert!(matches!(sent, Event::Sent { content: ContentKind::Image, .. }));
    let receiving = windows.wait_for(|event| matches!(event, Event::Receiving { .. } | Event::Received { .. })).await;
    assert!(matches!(receiving, Event::Receiving { content: ContentKind::Image, .. }));
    windows.copy.send(image).await.unwrap();
    mac.assert_nothing_written(Duration::from_millis(400)).await;

    // Görselden sonra bağlantı sağlıklı: metin de geçer, kopma olmadı.
    windows.copy.send(text("sonra")).await.unwrap();
    assert_eq!(mac.next_written().await, text("sonra"));
    while let Ok(event) = windows.events.try_recv() {
        assert!(!matches!(event, Event::Disconnected { .. }), "görsel aktarımı bağlantıyı koparmamalı");
    }
}

#[tokio::test]
async fn pause_blocks_sending_and_applying() {
    let (mut mac, windows) = paired_and_connected().await;
    mac.handle.send(Command::Pause);
    mac.wait_state(|snapshot| snapshot.paused).await;

    mac.copy.send(text("gizli not")).await.unwrap();
    windows.assert_nothing_written(Duration::from_millis(400)).await;
    windows.copy.send(text("windows kopyası")).await.unwrap();
    mac.wait_for(|event| matches!(event, Event::Skipped { .. })).await;
    mac.assert_nothing_written(Duration::from_millis(200)).await;

    mac.handle.send(Command::Resume);
    mac.wait_state(|snapshot| !snapshot.paused).await;
    mac.copy.send(text("devam")).await.unwrap();
    assert_eq!(windows.next_written().await, text("devam"));
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
    mac.copy.send(text("yeniden")).await.unwrap();
    assert_eq!(windows.next_written().await, text("yeniden"));
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
