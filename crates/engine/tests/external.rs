//! Kabuğun kendi keşfi (iOS Bonjour): yayın işareti geri çağrıya gider,
//! kabuğun bulduğu cihazlar motora ulaşır.

mod common;

use std::sync::{Arc, Mutex};

use common::{Node, fast_timing, free_port, pair};
use oc_engine::{Command, DiscoveryMode, Event, ExternalDiscovery, Found, PairingStage};
use tokio::sync::mpsc;

fn external_node(name: &str) -> (Node, mpsc::Sender<Found>, Arc<Mutex<Vec<bool>>>) {
    let flags = Arc::new(Mutex::new(Vec::new()));
    let recorded = flags.clone();
    let (found, receiver) = mpsc::channel(4);
    let discovery = DiscoveryMode::External(ExternalDiscovery {
        found: receiver,
        set_pairing: Box::new(move |on| recorded.lock().unwrap().push(on)),
    });
    let node = Node::start_with(name, tempfile::tempdir().unwrap(), free_port(), fast_timing(), discovery);
    (node, found, flags)
}

#[tokio::test]
async fn pairing_flag_goes_to_shell() {
    let (mut iphone, _found, flags) = external_node("iphone");
    iphone.handle.send(Command::StartPairing { address: None });
    iphone.wait_state(|snapshot| matches!(snapshot.pairing, Some(PairingStage::Searching { .. }))).await;
    iphone.handle.send(Command::CancelPairing);
    iphone.wait_state(|snapshot| snapshot.pairing.is_none()).await;
    assert_eq!(*flags.lock().unwrap(), vec![true, false]);
}

#[tokio::test]
async fn shell_found_reaches_group_member() {
    let (mut iphone, found, _flags) = external_node("iphone");
    let mut mac = Node::start("mac");
    pair(&mut mac, &mut iphone).await;
    let identity = oc_pairing::Identity::load_or_create(mac.dir.path(), "x").unwrap();
    let address = format!("127.0.0.1:{}", mac.port).parse().unwrap();
    found
        .send(Found { name: "mac".into(), fingerprint_hex: hex::encode(identity.fingerprint()), pairing: false, addresses: vec![address] })
        .await
        .unwrap();
    iphone
        .wait_for(|event| matches!(event, Event::Diagnostic { message } if message.contains("ağda bulundu")))
        .await;
    iphone.wait_connected().await;
}
