//! Sistem panosuna dokunmayan test eşi (iOS simülatörüyle denemek için).
//! Gerçek OpenContinuity açıkken aynı Mac'te ayrı kimlik ve portla çalışır:
//!
//!   OC_CONFIG_DIR=/tmp/oc-fake OC_PORT=47810 cargo run -p oc-engine --example fake_peer
//!
//! Komutlar (stdin): `pair`, `e` / `h` (kod onayı), `text <metin>`,
//! `png <dosya>`, `files <yol> [yol…]`, `quit`. Gelen içerik yazılmaz;
//! yalnız tür, uzunluk ve SHA-256 öneki.

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

use oc_engine::{Clip, ClipboardPort, ClipboardSource, Command, Engine, EngineConfig, Event};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = EngineConfig::system()?;
    config.files_dir = config.config_dir.join("incoming");
    let (copies, local_copies) = mpsc::channel(8);
    let (writes, written) = std_mpsc::channel::<Clip>();
    let (engine, handle, mut events) = Engine::new(config, ClipboardSource::Port(ClipboardPort { local_copies, writes }));
    let task = tokio::spawn(engine.run());

    std::thread::spawn(move || {
        while let Ok(clip) = written.recv() {
            println!("PANOYA YAZILDI {}", describe(&clip));
        }
    });
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            match event {
                Event::State(snapshot) => {
                    let peers: Vec<String> =
                        snapshot.peers.iter().map(|peer| format!("{}={}", peer.name, peer.connected)).collect();
                    println!("DURUM eşler=[{}] eşleştirme={:?}", peers.join(", "), snapshot.pairing);
                }
                Event::Diagnostic { message } => println!("TEŞHİS {message}"),
                other => println!("OLAY {other:?}"),
            }
        }
    });

    let stdin_handle = handle.clone();
    tokio::task::spawn_blocking(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            let (verb, rest) = line.trim().split_once(' ').unwrap_or((line.trim(), ""));
            match verb {
                "pair" => stdin_handle.send(Command::StartPairing { address: None }),
                "e" => stdin_handle.send(Command::ConfirmCode(true)),
                "h" => stdin_handle.send(Command::ConfirmCode(false)),
                "text" => send(&copies, Clip::Text(rest.to_owned())),
                "png" => match std::fs::read(rest) {
                    Ok(bytes) => send(&copies, Clip::Png(bytes)),
                    Err(error) => println!("HATA {rest}: {error}"),
                },
                "files" => send(&copies, Clip::Files(rest.split_whitespace().map(PathBuf::from).collect())),
                "quit" => break,
                _ => println!("komutlar: pair | e | h | text <metin> | png <dosya> | files <yol…> | quit"),
            }
        }
        stdin_handle.send(Command::Shutdown);
    });
    task.await?
}

fn send(copies: &mpsc::Sender<Clip>, clip: Clip) {
    println!("GÖNDERİLDİ {}", describe(&clip));
    let _ = copies.blocking_send(clip);
}

fn describe(clip: &Clip) -> String {
    match clip {
        Clip::Text(text) => format!("metin {} bayt {}", text.len(), hash(text.as_bytes())),
        Clip::Png(bytes) => format!("png {} bayt {}", bytes.len(), hash(bytes)),
        Clip::Files(paths) => format!("dosyalar {} öğe", paths.len()),
    }
}

fn hash(bytes: &[u8]) -> String {
    hex::encode(&Sha256::digest(bytes)[..6])
}
