use std::collections::VecDeque;

use super::*;

const MAC: DeviceId = [1; 16];
const WINDOWS: DeviceId = [2; 16];

fn text(value: &str) -> Payload {
    Payload::Text(value.to_owned())
}

#[test]
fn local_copy_reaches_peer_and_echo_is_suppressed() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);

    let event = mac.on_local(text("merhaba")).unwrap();
    assert!(windows.on_remote(&MAC, &event));
    // Windows panoya yazdı, izleyicisi değişikliği gördü: geri gönderilmemeli.
    assert!(windows.on_local(text("merhaba")).is_none());
}

#[test]
fn same_text_copied_again_after_echo_is_sent() {
    let mut windows = SyncState::new(WINDOWS);
    let mut mac = SyncState::new(MAC);
    let event = mac.on_local(text("a")).unwrap();
    windows.on_remote(&MAC, &event);
    assert!(windows.on_local(text("a")).is_none());
    assert!(windows.on_local(text("a")).is_some());
}

#[test]
fn duplicate_event_is_applied_once() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let event = mac.on_local(text("x")).unwrap();
    assert!(windows.on_remote(&MAC, &event));
    assert!(!windows.on_remote(&MAC, &event));
}

#[test]
fn delayed_remote_does_not_overwrite_newer_local_copy() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let first = mac.on_local(text("eski")).unwrap();
    assert!(windows.on_remote(&MAC, &first));
    // Windows kendi kopyasını yaptı; Mac'in daha eski (aynı ya da düşük saatli) olayı gecikmiş geliyor.
    let windows_copy = windows.on_local(text("yeni")).unwrap();
    let mut late = mac.on_local(text("gecikmiş")).unwrap();
    late.lamport = windows_copy.lamport - 1;
    assert!(!windows.on_remote(&MAC, &late));
}

#[test]
fn concurrent_copies_converge_to_smaller_device_id() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let from_mac = mac.on_local(text("mac")).unwrap();
    let from_windows = windows.on_local(text("windows")).unwrap();
    assert_eq!(from_mac.lamport, from_windows.lamport);
    assert!(windows.on_remote(&MAC, &from_mac), "küçük kimlik kazanır");
    assert!(!mac.on_remote(&WINDOWS, &from_windows));
}

/// İki sahte pano + gecikmeli kuyruklarla 1.000 olaylık rastgele senaryo.
#[test]
fn thousand_random_events_no_loop_and_converge() {
    let mut nodes = [Node::new(MAC), Node::new(WINDOWS)];
    let mut queues: [VecDeque<ClipEvent>; 2] = [VecDeque::new(), VecDeque::new()];
    let mut random = Lcg(0x5eed);
    let mut local_copies = 0;
    let mut messages_sent = 0;

    for step in 0..1_000 {
        let index = (random.next() % 2) as usize;
        match random.next() % 3 {
            0 => {
                local_copies += 1;
                nodes[index].clipboard = Payload::Text(format!("kopya {step} 🇹🇷"));
                if let Some(event) = nodes[index].poll_clipboard() {
                    messages_sent += 1;
                    queues[1 - index].push_back(event);
                }
            }
            _ => deliver_one(&mut nodes, &mut queues, index, &mut messages_sent),
        }
    }
    let mut drain_rounds = 0;
    while !queues[0].is_empty() || !queues[1].is_empty() {
        drain_rounds += 1;
        assert!(drain_rounds <= 2_000, "kuyruk boşalmıyor: yankı döngüsü");
        for index in 0..2 {
            deliver_one(&mut nodes, &mut queues, index, &mut messages_sent);
        }
    }

    assert!(messages_sent <= local_copies, "yankı döngüsü: {messages_sent} > {local_copies}");
    assert_eq!(nodes[0].clipboard, nodes[1].clipboard, "son durum tutarlı olmalı");
}

fn deliver_one(nodes: &mut [Node; 2], queues: &mut [VecDeque<ClipEvent>; 2], index: usize, sent: &mut usize) {
    let Some(event) = queues[index].pop_front() else { return };
    let sender = nodes[1 - index].state.device_id;
    let node = &mut nodes[index];
    if node.state.on_remote(&sender, &event) {
        node.clipboard = event.payload;
        if let Some(echo) = node.poll_clipboard() {
            *sent += 1;
            queues[1 - index].push_back(echo);
        }
    }
}

struct Node {
    state: SyncState,
    clipboard: Payload,
}

impl Node {
    fn new(device_id: DeviceId) -> Self {
        Self { state: SyncState::new(device_id), clipboard: text("") }
    }

    fn poll_clipboard(&mut self) -> Option<ClipEvent> {
        self.state.on_local(self.clipboard.clone())
    }
}

/// Bağımlılıksız, tekrarlanabilir rastgele sayı üreteci.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// Gerçek cihaz testinde görüldü: Windows `oc run` yeniden başlayınca saati 0'dan
/// başladı, ilk kopyası (lamport 1) Mac'in 7 dk önceki kopyasıyla (lamport 1)
/// eşit sayılıp reddedildi.
#[test]
fn restarted_peer_first_copy_is_applied_after_clock_sync() {
    let mut mac = SyncState::new(MAC);
    mac.on_local(text("7 dk önceki Mac kopyası")).unwrap();

    let mut restarted_windows = SyncState::new(WINDOWS);
    restarted_windows.observe_peer_clock(mac.clock());
    mac.observe_peer_clock(restarted_windows.clock());

    let event = restarted_windows.on_local(text("yeni Windows kopyası")).unwrap();
    assert!(mac.on_remote(&WINDOWS, &event));
}

#[test]
fn image_echo_is_suppressed_and_text_with_same_bytes_is_not() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let png = Payload::Png(b"\x89PNG\r\n\x1a\nveri".to_vec());

    let event = mac.on_local(png.clone()).unwrap();
    assert!(windows.on_remote(&MAC, &event));
    assert!(windows.on_local(png).is_none(), "panoya yazılan görsel geri gönderilmemeli");

    let event = mac.on_local(text("aynı")).unwrap();
    assert!(windows.on_remote(&MAC, &event));
    assert!(windows.on_local(Payload::Png("aynı".as_bytes().to_vec())).is_some(), "farklı tür yankı sayılmamalı");
}

const LINUX: DeviceId = [3; 16];

/// Üç cihazlı tam ağ: her kopya diğer ikisine ayrı kuyruklarla, farklı sırada
/// ulaşır. Eski kural (yalnız yerel kopyayla karşılaştırma) burada cihazları
/// farklı içerikte bırakıyordu.
#[test]
fn three_devices_random_events_no_loop_and_converge() {
    let ids = [MAC, WINDOWS, LINUX];
    let mut nodes = ids.map(Node::new);
    // queues[to][from]
    let mut queues: [[VecDeque<ClipEvent>; 3]; 3] = Default::default();
    let mut random = Lcg(0xc0ffee);
    let mut local_copies = 0;
    let mut messages_sent = 0;

    for step in 0..3_000 {
        let index = (random.next() % 3) as usize;
        if random.next().is_multiple_of(3) {
            local_copies += 1;
            nodes[index].clipboard = Payload::Text(format!("kopya {step}"));
            if let Some(event) = nodes[index].poll_clipboard() {
                broadcast(&mut queues, index, event, &mut messages_sent);
            }
        } else {
            let from = (random.next() % 3) as usize;
            deliver_from(&mut nodes, &mut queues, &ids, index, from, &mut messages_sent);
        }
    }
    let mut drain_rounds = 0;
    while queues.iter().flatten().any(|queue| !queue.is_empty()) {
        drain_rounds += 1;
        assert!(drain_rounds <= 5_000, "kuyruk boşalmıyor: yankı döngüsü");
        for to in 0..3 {
            for from in 0..3 {
                deliver_from(&mut nodes, &mut queues, &ids, to, from, &mut messages_sent);
            }
        }
    }

    assert!(messages_sent <= local_copies * 2, "yankı döngüsü: {messages_sent} > {}", local_copies * 2);
    assert_eq!(nodes[0].clipboard, nodes[1].clipboard, "Mac ve Windows aynı içerikte olmalı");
    assert_eq!(nodes[1].clipboard, nodes[2].clipboard, "Windows ve Linux aynı içerikte olmalı");
}

fn broadcast(queues: &mut [[VecDeque<ClipEvent>; 3]; 3], from: usize, event: ClipEvent, sent: &mut usize) {
    for (to, inbox) in queues.iter_mut().enumerate() {
        if to != from {
            *sent += 1;
            inbox[from].push_back(event.clone());
        }
    }
}

fn deliver_from(
    nodes: &mut [Node; 3],
    queues: &mut [[VecDeque<ClipEvent>; 3]; 3],
    ids: &[DeviceId; 3],
    to: usize,
    from: usize,
    sent: &mut usize,
) {
    let Some(event) = queues[to][from].pop_front() else { return };
    let node = &mut nodes[to];
    if node.state.on_remote(&ids[from], &event) {
        node.clipboard = event.payload;
        if let Some(echo) = node.poll_clipboard() {
            broadcast(queues, to, echo, sent);
        }
    }
}
