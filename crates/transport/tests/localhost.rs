use oc_pairing::{Identity, sas_code};
use oc_protocol::{ClipEvent, Message, Payload};
use oc_transport::{Session, Trust, accept, connect};
use tokio::net::TcpListener;

fn identity(name: &str) -> Identity {
    Identity::generate(name.to_owned()).unwrap()
}

/// Sunucu `server_trust`, istemci `client_trust` ile bağlanır.
async fn pair_up(
    server: &Identity,
    client: &Identity,
    server_trust: Trust,
    client_trust: Trust,
) -> (anyhow::Result<Session>, anyhow::Result<Session>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = server.clone();
    let accepting = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        accept(tcp, &server, server_trust, SERVER_PORT).await
    });
    let connected = connect(address, client, client_trust, CLIENT_PORT).await;
    (accepting.await.unwrap(), connected)
}

const SERVER_PORT: u16 = 47800;
const CLIENT_PORT: u16 = 47801;

#[tokio::test]
async fn pairing_both_sides_see_same_code() {
    let (mac, windows) = (identity("Mac"), identity("Windows"));
    let (server, client) = pair_up(&mac, &windows, Trust::Pairing, Trust::Pairing).await;
    let (server, client) = (server.unwrap(), client.unwrap());

    assert_eq!(server.peer_fingerprint, windows.fingerprint());
    assert_eq!(client.peer_fingerprint, mac.fingerprint());
    assert_eq!(server.peer_name, "Windows");
    assert_eq!(server.exporter, client.exporter);
    assert_eq!(
        sas_code(&mac.fingerprint(), &server.peer_fingerprint, &server.exporter),
        sas_code(&windows.fingerprint(), &client.peer_fingerprint, &client.exporter),
    );
}

#[tokio::test]
async fn each_session_has_different_exporter() {
    let (mac, windows) = (identity("Mac"), identity("Windows"));
    let (first, _) = pair_up(&mac, &windows, Trust::Pairing, Trust::Pairing).await;
    let (second, _) = pair_up(&mac, &windows, Trust::Pairing, Trust::Pairing).await;
    assert_ne!(first.unwrap().exporter, second.unwrap().exporter);
}

#[tokio::test]
async fn pinned_peers_connect_and_exchange_text() {
    let (mac, windows) = (identity("Mac"), identity("Windows"));
    let (server, client) =
        pair_up(&mac, &windows, Trust::Pinned(vec![windows.fingerprint()]), Trust::Pinned(vec![mac.fingerprint()])).await;
    let (mut server, mut client) = (server.unwrap(), client.unwrap());
    assert_eq!(server.peer_listen_port, CLIENT_PORT);
    assert_eq!(client.peer_listen_port, SERVER_PORT);

    let text = "Türkçe ğüşıöç 🇹🇷\r\nikinci satır".to_owned();
    let message = Message::Clip(ClipEvent { event_id: [1; 16], lamport: 1, payload: Payload::Text(text) });
    client.send(&message).await.unwrap();
    assert_eq!(server.recv().await.unwrap(), message);
}

#[tokio::test]
async fn unknown_client_is_rejected_by_pinned_server() {
    let (mac, windows, stranger) = (identity("Mac"), identity("Windows"), identity("Yabancı"));
    let (server, client) =
        pair_up(&mac, &stranger, Trust::Pinned(vec![windows.fingerprint()]), Trust::Pinned(vec![mac.fingerprint()])).await;
    assert!(server.is_err(), "sunucu yabancıyı kabul etmemeli");
    assert!(client.is_err());
}

#[tokio::test]
async fn unknown_server_is_rejected_by_pinned_client() {
    let (mac, windows, stranger) = (identity("Mac"), identity("Windows"), identity("Yabancı"));
    let (server, client) = pair_up(&stranger, &windows, Trust::Pairing, Trust::Pinned(vec![mac.fingerprint()])).await;
    assert!(client.is_err(), "istemci sahte sunucuya bağlanmamalı");
    assert!(server.is_err());
}

/// Eşleşmesi kaldırılmış cihazın eski eşi, eşleştirme bekleyen tarafla oturum kuramaz.
#[tokio::test]
async fn pairing_and_sync_modes_do_not_mix() {
    let (mac, windows) = (identity("Mac"), identity("Windows"));
    let (server, client) = pair_up(&mac, &windows, Trust::Pairing, Trust::Pinned(vec![mac.fingerprint()])).await;
    assert!(server.is_err(), "eşleştirme bekleyen taraf senkron modundaki eşi kabul etmemeli");
    assert!(client.is_err());
}

/// Kabul eden taraf gruptaki her eşi tanır; listede olmayan yine reddedilir.
#[tokio::test]
async fn pinned_server_accepts_any_group_member() {
    let (mac, windows, linux) = (identity("Mac"), identity("Windows"), identity("Linux"));
    let group = Trust::Pinned(vec![windows.fingerprint(), linux.fingerprint()]);
    let (server, client) = pair_up(&mac, &linux, group, Trust::Pinned(vec![mac.fingerprint()])).await;
    assert_eq!(server.unwrap().peer_name, "Linux");
    assert!(client.is_ok());
}
