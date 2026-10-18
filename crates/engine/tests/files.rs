mod common;

use std::path::Path;
use std::time::Duration;

use common::{paired_and_connected, text};
use oc_engine::{Clip, ContentKind, Event};

fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                found.push((relative + "/", Vec::new()));
                stack.push(path);
            } else {
                found.push((relative, std::fs::read(&path).unwrap()));
            }
        }
    }
    found.sort();
    found
}

#[tokio::test]
async fn files_and_folders_sync_and_echo_is_suppressed() {
    let (mut mac, windows) = paired_and_connected().await;
    let source = tempfile::tempdir().unwrap();
    let folder = source.path().join("Hasta Raporu");
    std::fs::create_dir_all(folder.join("boş klasör")).unwrap();
    std::fs::write(folder.join("rapor.pdf"), vec![42u8; 700_000]).unwrap();
    std::fs::write(folder.join("not.txt"), "Çağrı").unwrap();
    let single = source.path().join("fatura.xlsx");
    std::fs::write(&single, b"xlsx").unwrap();

    mac.copy.send(Clip::Files(vec![folder.clone(), single.clone()])).await.unwrap();
    let Clip::Files(written) = windows.next_written().await else { panic!("dosya bekleniyordu") };
    assert_eq!(written.len(), 2);
    assert_eq!(tree(&written[0]), tree(&folder));
    assert_eq!(std::fs::read(&written[1]).unwrap(), b"xlsx");
    assert!(written[0].starts_with(windows.files.path()));
    let sent = mac.wait_for(|event| matches!(event, Event::Sent { .. })).await;
    assert!(matches!(sent, Event::Sent { content: ContentKind::Files, items: 2, bytes: 700_012, .. }));

    // Alan taraf panoya yazdığı yolları yerel kopya olarak görür; geri gönderilmez.
    windows.copy.send(Clip::Files(written)).await.unwrap();
    mac.assert_nothing_written(Duration::from_millis(400)).await;
}

#[tokio::test]
async fn newer_copy_supersedes_transfer_and_leaves_no_partial() {
    let (mac, windows) = paired_and_connected().await;
    let source = tempfile::tempdir().unwrap();
    let big = source.path().join("büyük.bin");
    std::fs::write(&big, vec![1u8; 40 * 1024 * 1024]).unwrap();

    mac.copy.send(Clip::Files(vec![big])).await.unwrap();
    mac.copy.send(text("son kopya")).await.unwrap();
    // Dosya bitmeden metin gelirse dosya bırakılır; bittiyse önce dosya yazılır. Her
    // durumda son yazılan metindir ve yarım klasör kalmaz.
    let mut last = windows.next_written().await;
    if matches!(last, Clip::Files(_)) {
        last = windows.next_written().await;
    }
    assert_eq!(last, text("son kopya"));
    let leftovers: Vec<_> = std::fs::read_dir(windows.files.path())
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().file_name().to_string_lossy().starts_with(".partial-"))
        .collect();
    assert!(leftovers.is_empty(), "yarım aktarım klasörü kaldı");
}

#[tokio::test]
async fn oversized_copy_is_reported_not_sent() {
    let (mut mac, windows) = paired_and_connected().await;
    let source = tempfile::tempdir().unwrap();
    let huge = source.path().join("yedek.zip");
    // Seyrek dosya: diskte yer kaplamaz, boyutu sınırın üstünde.
    std::fs::File::create(&huge).unwrap().set_len(oc_protocol::MAX_FILES_BYTES + 1).unwrap();

    mac.copy.send(Clip::Files(vec![huge])).await.unwrap();
    let event = mac.wait_for(|event| matches!(event, Event::TooLarge { .. })).await;
    assert!(matches!(event, Event::TooLarge { content: ContentKind::Files, .. }));
    windows.assert_nothing_written(Duration::from_millis(300)).await;
}
