//! İki motoru aynı makinede, sahte panoyla ve mDNS'siz çalıştıran yardımcılar.
// Her test dosyası yardımcıların bir kısmını kullanır.
#![allow(dead_code)]

use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use oc_engine::{
    Clip, ClipboardPort, ClipboardSource, Command, Engine, EngineConfig, Event, Handle, PairingStage, Snapshot, Timing,
};
use tokio::sync::mpsc;
use tokio::time::timeout;

pub const WAIT: Duration = Duration::from_secs(10);

pub fn fast_timing() -> Timing {
    Timing {
        heartbeat_interval: Duration::from_millis(100),
        heartbeat_timeout: Duration::from_millis(600),
        handshake_timeout: Duration::from_secs(2),
        dial_timeout: Duration::from_secs(1),
        min_backoff: Duration::from_millis(50),
        max_backoff: Duration::from_millis(400),
        offline_copy_max_age: Duration::from_secs(120),
    }
}

pub fn text(value: &str) -> Clip {
    Clip::Text(value.to_owned())
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

pub struct Node {
    pub handle: Handle,
    pub events: mpsc::UnboundedReceiver<Event>,
    /// Sahte panoda "kullanıcı kopyaladı".
    pub copy: mpsc::Sender<Clip>,
    /// Motorun panoya yazdıkları.
    pub written: std_mpsc::Receiver<Clip>,
    pub port: u16,
    pub dir: tempfile::TempDir,
    /// Gelen dosyaların kaydedildiği klasör.
    pub files: tempfile::TempDir,
    task: tokio::task::JoinHandle<()>,
}

impl Node {
    pub fn start(name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        Self::start_in(name, dir)
    }

    pub fn start_in(name: &str, dir: tempfile::TempDir) -> Self {
        Self::start_at(name, dir, free_port(), fast_timing())
    }

    /// Yeniden başlatmayı taklit eder: aynı dizin ve port, istenen zamanlama.
    pub fn start_at(name: &str, dir: tempfile::TempDir, port: u16, timing: Timing) -> Self {
        let files = tempfile::tempdir().unwrap();
        let (copy, local_copies) = mpsc::channel(8);
        let (writes, written) = std_mpsc::channel();
        let config = EngineConfig {
            config_dir: dir.path().to_path_buf(),
            device_name: name.to_owned(),
            port,
            discovery: false,
            timing,
            files_dir: files.path().to_path_buf(),
        };
        let (engine, handle, events) = Engine::new(config, ClipboardSource::Port(ClipboardPort { local_copies, writes }));
        let task = tokio::spawn(async move { engine.run().await.unwrap() });
        Self { handle, events, copy, written, port, dir, files, task }
    }

    /// Motoru kapatır ve bitmesini bekler; aynı dizin ve portla yeniden
    /// başlatılabilsin. Sahte pano kanalı motor bitene kadar açık tutulur,
    /// yoksa motor "pano iş parçacığı durdu" diye hatayla çıkabilir.
    pub async fn stop(self) -> (tempfile::TempDir, u16) {
        self.handle.send(Command::Shutdown);
        timeout(WAIT, self.task).await.expect("motor kapanmadı").expect("motor hatayla bitti");
        (self.dir, self.port)
    }

    /// Koşulu sağlayan olay gelene kadar bekler; diğer olaylar atlanır.
    pub async fn wait_for(&mut self, mut predicate: impl FnMut(&Event) -> bool) -> Event {
        timeout(WAIT, async {
            loop {
                let event = self.events.recv().await.expect("motor durdu");
                if predicate(&event) {
                    return event;
                }
            }
        })
        .await
        .expect("beklenen olay gelmedi")
    }

    pub async fn wait_state(&mut self, mut predicate: impl FnMut(&Snapshot) -> bool) -> Snapshot {
        match self.wait_for(|event| matches!(event, Event::State(snapshot) if predicate(snapshot))).await {
            Event::State(snapshot) => snapshot,
            _ => unreachable!(),
        }
    }

    /// En az `count` eşe bağlı olana kadar bekler.
    pub async fn wait_connected_to(&mut self, count: usize) {
        self.wait_state(|snapshot| snapshot.connected_count() >= count).await;
    }

    pub async fn wait_connected(&mut self) {
        self.wait_state(|snapshot| snapshot.connected_count() > 0).await;
    }

    /// Panoya yazılan sonraki içerik.
    pub async fn next_written(&self) -> Clip {
        let deadline = std::time::Instant::now() + WAIT;
        loop {
            if let Ok(text) = self.written.try_recv() {
                return text;
            }
            assert!(std::time::Instant::now() < deadline, "panoya yazılmadı");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// Belirtilen süre boyunca panoya hiçbir şey yazılmadığını doğrular.
    pub async fn assert_nothing_written(&self, period: Duration) {
        tokio::time::sleep(period).await;
        assert!(self.written.try_recv().is_err(), "beklenmeyen pano yazımı");
    }
}

pub async fn code_of(node: &mut Node) -> String {
    match node.wait_state(|snapshot| matches!(snapshot.pairing, Some(PairingStage::Confirm { .. }))).await.pairing {
        Some(PairingStage::Confirm { code, .. }) => code,
        _ => unreachable!(),
    }
}

/// `waiting` dinler, `joining` ona adresle bağlanır; iki uç da kodu onaylar.
pub async fn pair(waiting: &mut Node, joining: &mut Node) {
    waiting.handle.send(Command::StartPairing { address: None });
    waiting.wait_state(|snapshot| matches!(snapshot.pairing, Some(PairingStage::Searching { .. }))).await;
    joining.handle.send(Command::StartPairing { address: Some(format!("127.0.0.1:{}", waiting.port)) });
    let (code_a, code_b) = (code_of(waiting).await, code_of(joining).await);
    assert_eq!(code_a, code_b, "iki uçta kod aynı olmalı");
    waiting.handle.send(Command::ConfirmCode(true));
    joining.handle.send(Command::ConfirmCode(true));
    waiting.wait_for(|event| matches!(event, Event::Paired { .. })).await;
    joining.wait_for(|event| matches!(event, Event::Paired { .. })).await;
}

/// Bu düğümün gruptan `other`'ı kaldırmasını sağlar.
pub fn unpair(node: &Node, other: &Node) {
    let identity = oc_pairing::Identity::load_or_create(other.dir.path(), "x").unwrap();
    let fingerprint_prefix = hex::encode(identity.fingerprint())[..16].to_owned();
    node.handle.send(Command::Unpair { fingerprint_prefix });
}

pub async fn paired_and_connected() -> (Node, Node) {
    let (mut mac, mut windows) = (Node::start("mac"), Node::start("windows"));
    pair(&mut mac, &mut windows).await;
    mac.wait_connected().await;
    windows.wait_connected().await;
    (mac, windows)
}
