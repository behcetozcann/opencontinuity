mod common;

use std::time::Duration;

use common::{Node, fast_timing, pair, paired_and_connected, text, unpair};
use oc_engine::{Command, Event};
use oc_pairing::PeerStore;

/// Mac–Windows eşli; Linux yalnız Windows'la kod onaylar, Mac onu Windows'tan öğrenir.
async fn group_of_three() -> (Node, Node, Node) {
    let (mut mac, mut windows) = paired_and_connected().await;
    let mut linux = Node::start("linux");
    pair(&mut windows, &mut linux).await;
    mac.wait_for(|event| matches!(event, Event::PeerAdded { peer, via } if peer == "linux" && via == "windows")).await;
    mac.wait_connected_to(2).await;
    windows.wait_connected_to(2).await;
    linux.wait_connected_to(2).await;
    (mac, windows, linux)
}

#[tokio::test]
async fn third_device_joins_through_any_member_and_all_sync() {
    let (mac, windows, linux) = group_of_three().await;
    assert_eq!(PeerStore::new(mac.dir.path()).load_all().unwrap().len(), 2);
    assert_eq!(PeerStore::new(linux.dir.path()).load_all().unwrap().len(), 2);

    mac.copy.send(text("mac'ten")).await.unwrap();
    assert_eq!(windows.next_written().await, text("mac'ten"));
    assert_eq!(linux.next_written().await, text("mac'ten"));
    // Yazılan içerik izleyiciye yerel kopya olarak döner; yankı gitmemeli.
    windows.copy.send(text("mac'ten")).await.unwrap();
    linux.copy.send(text("mac'ten")).await.unwrap();
    mac.assert_nothing_written(Duration::from_millis(400)).await;

    linux.copy.send(text("linux'tan")).await.unwrap();
    assert_eq!(mac.next_written().await, text("linux'tan"));
    assert_eq!(windows.next_written().await, text("linux'tan"));
}

#[tokio::test]
async fn offline_member_gets_copy_on_return_while_others_keep_syncing() {
    let (mut mac, windows, linux) = group_of_three().await;
    let (linux_dir, linux_port) = (linux.dir, linux.port);
    linux.handle.send(Command::Shutdown);
    mac.wait_for(|event| matches!(event, Event::Disconnected { peer, .. } if peer == "linux")).await;

    mac.copy.send(text("linux yokken")).await.unwrap();
    assert_eq!(windows.next_written().await, text("linux yokken"));
    let linux = Node::start_at("linux", linux_dir, linux_port, fast_timing());
    assert_eq!(linux.next_written().await, text("linux yokken"));
}

#[tokio::test]
async fn removed_member_is_not_reintroduced_by_others() {
    let (mut mac, windows, linux) = group_of_three().await;
    unpair(&mac, &linux);
    mac.wait_state(|snapshot| snapshot.peers.len() == 1).await;

    // Windows yeniden başlayıp Mac'e bağlanınca grup listesini (Linux dahil) yeniden gönderir.
    let (windows_dir, windows_port) = (windows.dir, windows.port);
    windows.handle.send(Command::Shutdown);
    let windows = Node::start_at("windows", windows_dir, windows_port, fast_timing());
    mac.wait_for(|event| matches!(event, Event::Diagnostic { message } if message.starts_with("Bağlandı: windows"))).await;

    mac.copy.send(text("yalnız windows")).await.unwrap();
    assert_eq!(windows.next_written().await, text("yalnız windows"));
    linux.assert_nothing_written(Duration::from_millis(400)).await;
    let names: Vec<String> = PeerStore::new(mac.dir.path()).load_all().unwrap().into_iter().map(|peer| peer.name).collect();
    assert_eq!(names, ["windows"]);
}
