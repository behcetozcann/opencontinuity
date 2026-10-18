use std::io::BufRead;

use anyhow::bail;
use oc_engine::{ClipboardSource, Command, Engine, EngineConfig, Event, PairingStage};
use oc_pairing::{Identity, PeerStore};

pub fn show_identity() -> anyhow::Result<()> {
    let config = EngineConfig::system()?;
    let identity = Identity::load_or_create(&config.config_dir, &config.device_name)?;
    println!("Cihaz adı:  {}", identity.name);
    println!("Parmak izi: {}", hex::encode(identity.fingerprint()));
    let peers = PeerStore::new(&config.config_dir).load_all()?;
    if peers.is_empty() {
        println!("Grup:       eş yok");
    }
    for peer in peers {
        println!("Eş:         {} ({})", peer.name, &hex::encode(peer.fingerprint)[..16]);
    }
    Ok(())
}

pub async fn pair(address: Option<String>) -> anyhow::Result<()> {
    let config = EngineConfig::system()?;
    let (engine, handle, mut events) = Engine::new(config, ClipboardSource::System);
    let task = tokio::spawn(engine.run());
    handle.send(Command::StartPairing { address });
    while let Some(event) = events.recv().await {
        match event {
            Event::State(snapshot) => match snapshot.pairing {
                Some(PairingStage::Searching { local_ip }) => {
                    println!("Aynı ağda eşleştirme bekleyen cihaz aranıyor...");
                    if let Some(ip) = local_ip {
                        println!("Bulunamazsa diğer cihazda:  oc pair --addr {ip}");
                    }
                }
                Some(PairingStage::Confirm { peer, code }) => {
                    println!("\nEş: {peer}");
                    println!("Doğrulama kodu:  {code}\n");
                    let accepted = ask_yes_no("Karşı ekranda da aynı kod görünüyor mu? (e/h): ").await?;
                    handle.send(Command::ConfirmCode(accepted));
                }
                Some(PairingStage::WaitingPeer { .. }) => println!("Karşı cihazın onayı bekleniyor..."),
                None => {}
            },
            Event::Paired { peer } => {
                println!("Eşleşti: {peer}");
                handle.send(Command::Shutdown);
            }
            Event::PairingFailed { reason } => {
                handle.send(Command::Shutdown);
                task.await??;
                bail!(reason);
            }
            _ => {}
        }
    }
    task.await?
}

/// `prefix`: `oc id`'nin gösterdiği parmak izi öneki; tek eşe uymalı.
pub fn unpair(prefix: &str) -> anyhow::Result<()> {
    let store = PeerStore::new(&EngineConfig::system()?.config_dir);
    let prefix = prefix.trim().to_lowercase();
    let matches: Vec<_> =
        store.load_all()?.into_iter().filter(|peer| !prefix.is_empty() && hex::encode(peer.fingerprint).starts_with(&prefix)).collect();
    let [peer] = matches.as_slice() else {
        bail!("'{prefix}' tek bir eşe uymuyor ({} eş); `oc id` ile önekleri gör", matches.len());
    };
    store.remove(&peer.fingerprint)?;
    println!("'{}' gruptan kaldırıldı. Diğer cihazlarda da kaldırın; yoksa onlarla senkron sürer.", peer.name);
    Ok(())
}

async fn ask_yes_no(prompt: &str) -> anyhow::Result<bool> {
    print!("{prompt}");
    std::io::Write::flush(&mut std::io::stdout())?;
    let line = tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).map(|_| line)
    })
    .await??;
    Ok(matches!(line.trim().to_lowercase().as_str(), "e" | "evet" | "y" | "yes"))
}
