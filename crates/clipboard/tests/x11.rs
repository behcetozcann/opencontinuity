//! Gerçek X sunucusunda (CI'da `xvfb-run`) Linux pano adaptörü. İki ayrı
//! `system()` örneği iki ayrı X istemcisidir: biri yazar, öteki okur.
//! `DISPLAY` yoksa testler atlanır.
#![cfg(target_os = "linux")]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use oc_clipboard::{Clipboard, Content};

/// Testler aynı X panosunu paylaşır; paralel koşarlarsa birbirinin içeriğini ezer.
static SHARED_CLIPBOARD: Mutex<()> = Mutex::new(());

type Pair = (MutexGuard<'static, ()>, Box<dyn Clipboard>, Box<dyn Clipboard>);

fn pair() -> Option<Pair> {
    std::env::var_os("DISPLAY")?;
    let guard = SHARED_CLIPBOARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    Some((guard, oc_clipboard::system().unwrap(), oc_clipboard::system().unwrap()))
}

/// Sayaç, X sunucusunun bildirdiği değişiklikle artar; olay ayrı iş parçacığında işlenir.
fn wait_sequence_above(clipboard: &dyn Clipboard, before: u64) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while clipboard.sequence() <= before {
        assert!(Instant::now() < deadline, "sayaç artmadı");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn text_round_trips_between_clients_and_counter_moves() {
    let Some((_serial, writer, reader)) = pair() else { return };
    let before = reader.sequence();
    let text = "Çağrı ışığı 👨‍👩‍👧 🇹🇷\r\nikinci satır\tsekme";
    writer.write_text(text).unwrap();
    wait_sequence_above(reader.as_ref(), before);
    assert_eq!(reader.read().unwrap(), Content::Text(text.to_owned()));
    // Sahip kendi içeriğini de okuyabilmeli (yankı kontrolü bunu yapar).
    assert_eq!(writer.read().unwrap(), Content::Text(text.to_owned()));
}

/// Tek istekten büyük veri INCR ile parça parça gider (her iki yönde de bizim kod).
#[test]
fn large_text_and_png_use_incr_and_keep_bytes() {
    let Some((_serial, writer, reader)) = pair() else { return };
    let text: String = (0..3 * 1024 * 1024).map(|index| char::from(b'a' + (index % 26) as u8)).collect();
    writer.write_text(&text).unwrap();
    assert_eq!(reader.read().unwrap(), Content::Text(text));

    let png = png(5 * 1024 * 1024);
    writer.write_png(&png).unwrap();
    assert_eq!(reader.read().unwrap(), Content::Image(png), "PNG baytları değişmemeli");
}

#[test]
fn files_round_trip_as_paths() {
    let Some((_serial, writer, reader)) = pair() else { return };
    let files = vec![PathBuf::from("/tmp/oc test/Masaüstü raporu.pdf"), PathBuf::from("/tmp/klasör")];
    writer.write_files(&files).unwrap();
    assert_eq!(reader.read().unwrap(), Content::Files(files));
    assert!(reader.describe_source().contains("x-special/gnome-copied-files"));
}

/// Başka bir uygulamanın (xclip) panosu okunur, bizim yazdığımızı o okur.
#[test]
fn interoperates_with_xclip() {
    let Some((_serial, writer, reader)) = pair() else { return };
    if Command::new("xclip").arg("-version").stderr(Stdio::null()).status().is_err() {
        eprintln!("xclip yok; atlandı");
        return;
    }
    let mut xclip = Command::new("xclip").args(["-selection", "clipboard", "-i"]).stdin(Stdio::piped()).spawn().unwrap();
    std::io::Write::write_all(xclip.stdin.as_mut().unwrap(), "xclip'ten ğüş".as_bytes()).unwrap();
    drop(xclip.stdin.take());
    xclip.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if reader.read().unwrap() == Content::Text("xclip'ten ğüş".into()) {
            break;
        }
        assert!(Instant::now() < deadline, "xclip içeriği okunamadı");
        std::thread::sleep(Duration::from_millis(50));
    }

    writer.write_text("bizden şçö").unwrap();
    let output = Command::new("xclip").args(["-selection", "clipboard", "-o"]).output().unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "bizden şçö");
    let output = Command::new("xclip").args(["-selection", "clipboard", "-o", "-t", "TARGETS"]).output().unwrap();
    assert!(String::from_utf8(output.stdout).unwrap().contains("UTF8_STRING"));
}

fn png(length: usize) -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    let body = length - png.len() - 24;
    png.extend_from_slice(&(body as u32).to_be_bytes());
    png.extend_from_slice(b"tEXt");
    png.extend((0..body).map(|index| (index % 251) as u8));
    png.extend_from_slice(&[0; 4]);
    png.extend_from_slice(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]);
    png
}
