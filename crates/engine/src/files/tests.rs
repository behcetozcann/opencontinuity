use super::*;

fn ready(roots: &[PathBuf]) -> Outgoing {
    match collect(roots).unwrap() {
        Collected::Ready(outgoing) => outgoing,
        Collected::TooLarge { .. } => panic!("sınır aşılmamalı"),
    }
}

#[test]
fn collect_lists_tree_sorted_and_skips_junk() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Rapor");
    std::fs::create_dir_all(folder.join("boş")).unwrap();
    std::fs::write(folder.join("b.txt"), "bb").unwrap();
    std::fs::write(folder.join("a.pdf"), "a").unwrap();
    std::fs::write(folder.join(".DS_Store"), "x").unwrap();
    std::fs::write(dir.path().join("not:1.txt"), "123").unwrap();

    let outgoing = ready(&[folder.clone(), dir.path().join("not:1.txt")]);
    let paths: Vec<_> = outgoing.list.entries.iter().map(|entry| (entry.path.as_str(), entry.size, entry.is_dir)).collect();
    assert_eq!(
        paths,
        [("Rapor", 0, true), ("Rapor/a.pdf", 1, false), ("Rapor/b.txt", 2, false), ("Rapor/boş", 0, true), ("not_1.txt", 3, false)]
    );
    assert_eq!(outgoing.sources, [folder.join("a.pdf"), folder.join("b.txt"), dir.path().join("not:1.txt")]);
}

#[tokio::test]
async fn sink_writes_tree_and_accept_moves_it() {
    let source = tempfile::tempdir().unwrap();
    let folder = source.path().join("Klasör");
    std::fs::create_dir_all(folder.join("iç")).unwrap();
    std::fs::write(folder.join("iç/veri.bin"), vec![7u8; 300_000]).unwrap();
    std::fs::write(folder.join("boş.txt"), "").unwrap();
    let outgoing = ready(&[folder]);

    let root = tempfile::tempdir().unwrap();
    let mut sink = FileSink::new(root.path().to_path_buf());
    assert!(sink.start([1; 16], 4, outgoing.list.clone()).await.unwrap().is_none());
    let data = std::fs::read(&outgoing.sources[1]).unwrap();
    let (first, second) = data.split_at(100_000);
    assert!(sink.chunk(first).await.unwrap().is_none());
    let completed = sink.chunk(second).await.unwrap().expect("tamamlanmalı");
    assert_eq!((completed.lamport, &completed.list), (4, &outgoing.list));

    let (saved, tops) = accept(root.path(), &[1; 16], &completed.list).unwrap();
    assert_eq!(tops, [saved.join("Klasör")]);
    assert_eq!(std::fs::read(saved.join("Klasör/iç/veri.bin")).unwrap(), data);
    assert_eq!(std::fs::read(saved.join("Klasör/boş.txt")).unwrap(), b"");
    // Alan tarafta panodan geri okunan klasör aynı listeyi üretir: yankı tanınır.
    assert_eq!(ready(&tops).list, outgoing.list);
}

#[tokio::test]
async fn sink_rejects_excess_data_and_cancel_removes_partial() {
    let root = tempfile::tempdir().unwrap();
    let list = FileList { entries: vec![FileEntry { path: "a".into(), size: 2, is_dir: false }] };
    let mut sink = FileSink::new(root.path().to_path_buf());
    sink.start([2; 16], 1, list.clone()).await.unwrap();
    assert!(sink.chunk(b"abc").await.is_err());
    sink.cancel().await;
    assert!(!partial_dir(root.path(), &[2; 16]).exists());
    assert!(sink.chunk(b"a").await.is_err(), "iptalden sonra parça kabul edilmez");
}

#[test]
fn prune_keeps_newest_stamped_folders_and_ignores_others() {
    let root = tempfile::tempdir().unwrap();
    for day in 1..=12 {
        std::fs::create_dir(root.path().join(format!("2026-10-{day:02} 10.00.00"))).unwrap();
    }
    std::fs::create_dir(root.path().join("Benim klasörüm")).unwrap();
    prune(root.path(), &root.path().join("yeni"));
    let mut left: Vec<_> = std::fs::read_dir(root.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    left.sort();
    assert_eq!(left.len(), KEPT_TRANSFERS);
    assert_eq!(left[0], "2026-10-04 10.00.00");
    assert!(left.contains(&"Benim klasörüm".into()));
}
