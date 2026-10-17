use std::collections::VecDeque;

use super::*;

const MAC: DeviceId = [1; 16];
const WINDOWS: DeviceId = [2; 16];

#[test]
fn local_copy_reaches_peer_and_echo_is_suppressed() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);

    let event = mac.on_local_text("merhaba".into()).unwrap();
    assert!(windows.on_remote(&MAC, &event));
    // Windows panoya yazdı, izleyicisi değişikliği gördü: geri gönderilmemeli.
    assert!(windows.on_local_text("merhaba".into()).is_none());
}

#[test]
fn same_text_copied_again_after_echo_is_sent() {
    let mut windows = SyncState::new(WINDOWS);
    let mut mac = SyncState::new(MAC);
    let event = mac.on_local_text("a".into()).unwrap();
    windows.on_remote(&MAC, &event);
    assert!(windows.on_local_text("a".into()).is_none());
    assert!(windows.on_local_text("a".into()).is_some());
}

#[test]
fn duplicate_event_is_applied_once() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let event = mac.on_local_text("x".into()).unwrap();
    assert!(windows.on_remote(&MAC, &event));
    assert!(!windows.on_remote(&MAC, &event));
}

#[test]
fn delayed_remote_does_not_overwrite_newer_local_copy() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let first = mac.on_local_text("eski".into()).unwrap();
    assert!(windows.on_remote(&MAC, &first));
    // Windows kendi kopyasını yaptı; Mac'in daha eski (aynı ya da düşük saatli) olayı gecikmiş geliyor.
    let windows_copy = windows.on_local_text("yeni".into()).unwrap();
    let mut late = mac.on_local_text("gecikmiş".into()).unwrap();
    late.lamport = windows_copy.lamport - 1;
    assert!(!windows.on_remote(&MAC, &late));
}

#[test]
fn concurrent_copies_converge_to_smaller_device_id() {
    let mut mac = SyncState::new(MAC);
    let mut windows = SyncState::new(WINDOWS);
    let from_mac = mac.on_local_text("mac".into()).unwrap();
    let from_windows = windows.on_local_text("windows".into()).unwrap();
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
                nodes[index].clipboard = format!("kopya {step} 🇹🇷");
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
        node.clipboard = event.text;
        if let Some(echo) = node.poll_clipboard() {
            *sent += 1;
            queues[1 - index].push_back(echo);
        }
    }
}

struct Node {
    state: SyncState,
    clipboard: String,
}

impl Node {
    fn new(device_id: DeviceId) -> Self {
        Self { state: SyncState::new(device_id), clipboard: String::new() }
    }

    fn poll_clipboard(&mut self) -> Option<ClipEvent> {
        self.state.on_local_text(self.clipboard.clone())
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
