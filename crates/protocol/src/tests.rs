use super::*;

fn clip(text: &str) -> Message {
    Message::Clip(ClipEvent { event_id: [7; 16], lamport: 42, text: text.to_owned() })
}

async fn round_trip(message: &Message) -> Message {
    let frame = encode(message).unwrap();
    read_message(&mut frame.as_slice()).await.unwrap()
}

#[tokio::test]
async fn every_message_type_round_trips() {
    let messages = [
        Message::Hello { version: VERSION, device_id: [1; 16], name: "Behçet'in Mac'i".into() },
        Message::PairDecision { accepted: true },
        clip("merhaba"),
        Message::Ping { nonce: u64::MAX },
        Message::Pong { nonce: 0 },
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
        let Message::Clip(event) = round_trip(&clip(sample)).await else { panic!() };
        assert_eq!(event.text.as_bytes(), sample.as_bytes());
    }
}

#[tokio::test]
async fn text_limit_is_inclusive() {
    let at_limit = "a".repeat(MAX_TEXT_BYTES);
    let Message::Clip(event) = round_trip(&clip(&at_limit)).await else { panic!() };
    assert_eq!(event.text.len(), MAX_TEXT_BYTES);
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
}
