use std::io::BufRead;
use std::net::{IpAddr, SocketAddr, UdpSocket};

use anyhow::bail;
use oc_pairing::{Identity, Peer, PeerStore, sas_code};
use oc_protocol::Message;
use oc_transport::{DEFAULT_PORT, Session, Trust};
use tokio::net::TcpListener;

use crate::config;

pub fn show_identity() -> anyhow::Result<()> {
    let identity = config::identity()?;
    println!("Cihaz adı:  {}", identity.name);
    println!("Parmak izi: {}", hex::encode(identity.fingerprint()));
    match PeerStore::new(&config::dir()?).load()? {
        Some(peer) => println!("Eş:         {} ({})", peer.name, &hex::encode(peer.fingerprint)[..16]),
        None => println!("Eş:         yok"),
    }
    Ok(())
}

pub async fn pair(address: Option<String>) -> anyhow::Result<()> {
    let identity = config::identity()?;
    let store = PeerStore::new(&config::dir()?);
    if let Some(peer) = store.load()? {
        bail!("zaten '{}' ile eşleşmiş; önce `oc unpair` çalıştır", peer.name);
    }
    let (mut session, peer_address) = match address {
        Some(address) => {
            let address = with_default_port(&address);
            (oc_transport::connect(address.as_str(), &identity, Trust::Pairing).await?, address)
        }
        None => listen_once(&identity).await?,
    };

    let code = sas_code(&identity.fingerprint(), &session.peer_fingerprint, &session.exporter);
    println!("\nEş: {}", session.peer_name);
    println!("Doğrulama kodu:  {code}\n");
    let accepted = ask_yes_no("Karşı ekranda da aynı kod görünüyor mu? (e/h): ").await?;
    session.send(&Message::PairDecision { accepted }).await?;
    if !accepted {
        bail!("eşleştirme reddedildi; hiçbir şey kaydedilmedi");
    }
    println!("Karşı cihazın onayı bekleniyor...");
    let Message::PairDecision { accepted: peer_accepted } = session.recv().await? else {
        bail!("eş beklenmeyen mesaj gönderdi");
    };
    if !peer_accepted {
        bail!("karşı cihaz reddetti; hiçbir şey kaydedilmedi");
    }
    store.save(&Peer {
        name: session.peer_name.clone(),
        fingerprint: session.peer_fingerprint,
        last_address: Some(peer_address),
    })?;
    println!("Eşleşti: {}", session.peer_name);
    Ok(())
}

pub fn unpair() -> anyhow::Result<()> {
    if PeerStore::new(&config::dir()?).remove()? {
        println!("Eşleşme kaldırıldı.");
    } else {
        println!("Kayıtlı eş yok.");
    }
    Ok(())
}

async fn listen_once(identity: &Identity) -> anyhow::Result<(Session, String)> {
    let listener = TcpListener::bind(("0.0.0.0", DEFAULT_PORT)).await?;
    match local_ip() {
        Some(ip) => println!("Bekleniyor. Diğer cihazda çalıştır:  oc pair --addr {ip}"),
        None => println!("Bekleniyor (port {DEFAULT_PORT})."),
    }
    let (tcp, remote) = listener.accept().await?;
    let session = oc_transport::accept(tcp, identity, Trust::Pairing).await?;
    Ok((session, format!("{}:{DEFAULT_PORT}", remote.ip())))
}

fn with_default_port(address: &str) -> String {
    if address.parse::<SocketAddr>().is_ok() {
        address.to_owned()
    } else {
        format!("{address}:{DEFAULT_PORT}")
    }
}

/// Varsayılan rotanın yerel adresi; UDP `connect` paket göndermez.
fn local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    Some(socket.local_addr().ok()?.ip())
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
