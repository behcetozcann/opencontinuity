//! Asıl yön (küçük parmak izli taraf arar) engelliyken bağlantı: örneğin Linux'ta
//! ufw gelen bağlantıları reddeder, Linux dışarı arayabilir.
mod common;

use std::time::Duration;

use common::{Node, fast_timing, free_port, pair, paired_and_connected, text};
use oc_engine::Event;
use oc_pairing::{Fingerprint, Identity, PeerStore};

fn fingerprint_in(dir: &tempfile::TempDir) -> Fingerprint {
    Identity::load_or_create(dir.path(), "x").unwrap().fingerprint()
}

#[tokio::test]
async fn waiting_side_dials_when_preferred_direction_is_blocked() {
    let (mut first, mut second) = (Node::start("first"), Node::start("second"));
    pair(&mut first, &mut second).await;
    first.wait_connected().await;
    second.wait_connected().await;
    let (first_saved, second_saved) = (first.stop().await, second.stop().await);
    let ((dialer_dir, dialer_port), (blocked_dir, blocked_port)) =
        if fingerprint_in(&first_saved.0) < fingerprint_in(&second_saved.0) { (first_saved, second_saved) } else { (second_saved, first_saved) };

    // Arayan taraf karşıyı yalnız kapalı bir porttan bilir: gelen bağlantısı engelli bir eş gibi.
    let store = PeerStore::new(dialer_dir.path());
    let mut blocked_record = store.load_all().unwrap().pop().unwrap();
    blocked_record.last_address = Some(format!("127.0.0.1:{}", free_port()));
    store.upsert(&blocked_record).unwrap();

    let mut dialer = Node::start_at("dialer", dialer_dir, dialer_port, fast_timing());
    let mut blocked = Node::start_at("blocked", blocked_dir, blocked_port, fast_timing());
    dialer.wait_connected().await;
    blocked.wait_connected().await;

    dialer.copy.send(text("arayandan")).await.unwrap();
    assert_eq!(blocked.next_written().await, text("arayandan"));
    blocked.copy.send(text("engelliden")).await.unwrap();
    assert_eq!(dialer.next_written().await, text("engelliden"));
}

/// Asıl yön çalışırken yedek arama ikinci bağlantı açıp mevcut olanı koparmamalı.
#[tokio::test]
async fn fallback_does_not_disturb_working_connection() {
    let (mut mac, mut windows) = paired_and_connected().await;
    tokio::time::sleep(fast_timing().fallback_delay * 4).await;
    for node in [&mut mac, &mut windows] {
        while let Ok(event) = node.events.try_recv() {
            assert!(!matches!(event, Event::Disconnected { .. }), "çalışan bağlantı kopmamalı");
        }
    }
    mac.copy.send(text("tek bağlantı")).await.unwrap();
    assert_eq!(windows.next_written().await, text("tek bağlantı"));
    windows.assert_nothing_written(Duration::from_millis(200)).await;
}

/// Gecikme sıfırken iki taraf aynı anda arar: iki uç da asıl yöndeki bağlantıyı
/// tutmalı, bağlantı kopup açılma döngüsüne girmemeli. Yedek bağlantıyı önce
/// etkinleştiren uç, karşı taraf onu kapatınca bir kez "koptu" görebilir; bu
/// geçişten sonra kopma olmamalı.
#[tokio::test]
async fn simultaneous_dials_settle_on_one_connection() {
    let (mut first, mut second) = paired_and_connected().await;
    let eager = oc_engine::Timing { fallback_delay: Duration::ZERO, ..fast_timing() };
    let (first_dir, first_port) = first.stop().await;
    second.wait_current(|snapshot| snapshot.connected_count() == 0).await;
    let (second_dir, second_port) = second.stop().await;
    first = Node::start_at("first", first_dir, first_port, eager);
    second = Node::start_at("second", second_dir, second_port, eager);
    tokio::time::sleep(Duration::from_millis(300)).await;
    for node in [&mut first, &mut second] {
        node.wait_current(|snapshot| snapshot.connected_count() == 1).await;
        while node.events.try_recv().is_ok() {}
    }

    tokio::time::sleep(Duration::from_millis(800)).await;
    for node in [&mut first, &mut second] {
        while let Ok(event) = node.events.try_recv() {
            assert!(!matches!(event, Event::Disconnected { .. }), "bağlantı kopmamalı");
        }
    }
    first.copy.send(text("birinci")).await.unwrap();
    assert_eq!(second.next_written().await, text("birinci"));
    second.copy.send(text("ikinci")).await.unwrap();
    assert_eq!(first.next_written().await, text("ikinci"));
}
