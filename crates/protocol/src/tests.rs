use super::*;

fn clip(text: &str) -> Message {
    Message::Clip(ClipEvent { event_id: [7; 16], lamport: 42, payload: Payload::Text(text.to_owned()) })
}

fn png(length: usize) -> Vec<u8> {
    let mut png = PNG_SIGNATURE.to_vec();
    png.extend((0..length - PNG_SIGNATURE.len()).map(|index| (index % 251) as u8));
    png
}

fn image(length: usize) -> Message {
    Message::Clip(ClipEvent { event_id: [9; 16], lamport: 5, payload: Payload::Png(png(length)) })
}

fn text_of(message: Message) -> String {
    match message {
        Message::Clip(ClipEvent { payload: Payload::Text(text), .. }) => text,
        other => panic!("metin bekleniyordu: {other:?}"),
    }
}

async fn round_trip(message: &Message) -> Message {
    let frame = encode(message).unwrap();
    read_message(&mut frame.as_slice()).await.unwrap()
}

#[tokio::test]
async fn every_message_type_round_trips() {
    let messages = [
        Message::Hello {
            version: VERSION,
            device_id: [1; 16],
            name: "Behçet'in Mac'i".into(),
            listen_port: 47800,
            pairing: true,
        },
        Message::PairDecision { accepted: true },
        Message::Clock { lamport: 7 },
        clip("merhaba"),
        image(64),
        Message::ImageStart { event_id: [2; 16], lamport: 3, total_bytes: 10 },
        Message::ImageChunk { data: Chunk(vec![1, 2, 3]) },
        Message::Ping { nonce: u64::MAX },
        Message::Pong { nonce: 0 },
        Message::Peers {
            peers: vec![PeerInfo { name: "Linux".into(), fingerprint: [4; 32], address: Some("192.168.1.9:47800".into()) }],
        },
    ];
    for message in messages {
        assert_eq!(round_trip(&message).await, message);
    }
}

#[tokio::test]
async fn text_is_preserved_byte_for_byte() {
    let samples = [
        "",
        "ğüşıöç ĞÜŞİÖÇ",
        "👨‍👩‍👧‍👦 🇹🇷 é (e + birleşik aksan: e\u{301})",
        "satır1\r\nsatır2\nsatır3\r",
        "\t  baştaki ve sondaki boşluk  \t",
        "\u{0}null karakter",
    ];
    for sample in samples {
        assert_eq!(text_of(round_trip(&clip(sample)).await).as_bytes(), sample.as_bytes());
    }
}

#[tokio::test]
async fn text_limit_is_inclusive() {
    let at_limit = "a".repeat(MAX_TEXT_BYTES);
    assert_eq!(text_of(round_trip(&clip(&at_limit)).await).len(), MAX_TEXT_BYTES);
    assert!(encode(&clip(&"a".repeat(MAX_TEXT_BYTES + 1))).is_err());
}

#[tokio::test]
async fn oversized_frame_header_is_rejected_without_allocating() {
    let header = (u32::MAX).to_be_bytes();
    let error = read_message(&mut header.as_slice()).await.unwrap_err();
    assert!(error.to_string().contains("çok büyük"));
}

#[test]
fn corrupt_body_is_rejected() {
    assert!(decode_body(&[0xff, 0xff, 0xff]).is_err());
    assert!(decode_body(&[]).is_err());
}

#[test]
fn invalid_utf8_is_rejected() {
    let mut body = postcard::to_stdvec(&clip("ab")).unwrap();
    let last = body.len() - 1;
    body[last] = 0xff;
    assert!(decode_body(&body).is_err());
}

#[test]
fn debug_output_hides_clipboard_text() {
    let debug = format!("{:?}", clip("gizli-parola"));
    assert!(!debug.contains("gizli-parola"));
    let debug = format!("{:?}", Message::ImageChunk { data: Chunk(vec![42; 3]) });
    assert!(!debug.contains("42"));
}

#[tokio::test]
async fn image_larger_than_frame_survives_split_and_assembly() {
    let original = image(MAX_FRAME_BYTES * 2 + 17);
    let parts = split(original.clone());
    assert_eq!(parts.len(), 1 + (MAX_FRAME_BYTES * 2 + 17).div_ceil(IMAGE_CHUNK_BYTES));
    let mut assembler = Assembler::default();
    let mut completed = Vec::new();
    for part in parts {
        if let Some(message) = assembler.push(round_trip(&part).await).unwrap() {
            completed.push(message);
        }
    }
    assert_eq!(completed, vec![original]);
}

#[test]
fn other_messages_pass_through_split_and_assembler() {
    assert_eq!(split(clip("a")), vec![clip("a")]);
    let mut assembler = Assembler::default();
    assert_eq!(assembler.push(Message::Ping { nonce: 1 }).unwrap(), Some(Message::Ping { nonce: 1 }));
}

#[test]
fn image_limits_and_signature_are_enforced() {
    assert!(encode(&image(64)).is_ok());
    let fake = Message::Clip(ClipEvent { event_id: [0; 16], lamport: 1, payload: Payload::Png(vec![0; 64]) });
    assert!(encode(&fake).is_err(), "PNG imzası olmayan görsel reddedilmeli");
    let too_big = Message::ImageStart { event_id: [0; 16], lamport: 1, total_bytes: MAX_IMAGE_BYTES as u32 + 1 };
    assert!(encode(&too_big).is_err());
    assert!(encode(&Message::ImageChunk { data: Chunk(vec![0; IMAGE_CHUNK_BYTES + 1]) }).is_err());
}

#[test]
fn malformed_chunk_sequences_are_rejected() {
    let start = Message::ImageStart { event_id: [0; 16], lamport: 1, total_bytes: 16 };
    let chunk = |length| Message::ImageChunk { data: Chunk(png(16)[..length].to_vec()) };

    assert!(Assembler::default().push(chunk(8)).is_err(), "başlangıçsız parça");
    let mut assembler = Assembler::default();
    assembler.push(start.clone()).unwrap();
    assert!(assembler.push(start.clone()).is_err(), "iç içe başlangıç");
    let mut assembler = Assembler::default();
    assembler.push(start.clone()).unwrap();
    assembler.push(chunk(12)).unwrap();
    assert!(assembler.push(chunk(8)).is_err(), "bildirilen boyutu aşan parça");
    let mut assembler = Assembler::default();
    assembler.push(start).unwrap();
    assert!(assembler.push(Message::ImageChunk { data: Chunk(vec![0; 16]) }).is_err(), "PNG olmayan birleşim");
}

#[test]
fn oversized_group_is_rejected() {
    let peer = PeerInfo { name: "a".into(), fingerprint: [1; 32], address: None };
    assert!(encode(&Message::Peers { peers: vec![peer.clone(); MAX_GROUP_PEERS] }).is_ok());
    assert!(encode(&Message::Peers { peers: vec![peer.clone(); MAX_GROUP_PEERS + 1] }).is_err());
    let long_name = PeerInfo { name: "a".repeat(257), ..peer };
    assert!(encode(&Message::Peers { peers: vec![long_name] }).is_err());
}

/// Daha yeni sürümün eklediği mesaj bağlantıyı koparmaz: atlanır, sonraki mesaj okunur.
#[tokio::test]
async fn message_from_newer_version_is_skipped() {
    let unknown_body = [KNOWN_MESSAGES as u8, 1, 2, 3];
    let mut stream = (unknown_body.len() as u32).to_be_bytes().to_vec();
    stream.extend_from_slice(&unknown_body);
    stream.extend(encode(&clip("sonraki")).unwrap());
    assert_eq!(text_of(read_message(&mut stream.as_slice()).await.unwrap()), "sonraki");
}

/// `KNOWN_MESSAGES` son mesajla birlikte güncellenmeli; yoksa yeni mesaj atlanır.
#[test]
fn known_message_count_matches_last_variant() {
    let body = postcard::to_stdvec(&Message::Peers { peers: Vec::new() }).unwrap();
    assert_eq!(u32::from(body[0]) + 1, KNOWN_MESSAGES);
}
