# Mimari (EDD) — Faz 0

## Bileşenler

| Crate | Görev |
|---|---|
| `protocol` | Sürümlü mesaj tipleri ve çerçeve codec'i ([protocol.md](protocol.md)) |
| `core` | Senkron durum makinesi: Lamport sayacı, döngü/tekrar önleme, çakışma |
| `clipboard` | Ortak `Clipboard` arayüzü + macOS (NSPasteboard) ve Windows (Win32) adaptörleri |
| `pairing` | Cihaz kimliği (self-signed P-256), eş listesi, 6 haneli SAS |
| `transport` | Karşılıklı TLS 1.3 (rustls, ring), parmak izi sabitleme, heartbeat |
| `discovery` | mDNS / DNS-SD yayını ve taraması (`_opencontinuity._tcp.local.`) |
| `apps/cli` | `oc` binary'si; bileşenleri birbirine bağlar |

## Olay akışı

```text
yerel pano değişti ─► clipboard adaptörü ─► gizli/geçici mi? ── evet ─► at
                                              │ hayır
                                              ▼
                              core: son uygulanan uzak hash'e eşit mi? ── evet ─► at (döngü)
                                              │ hayır
                                              ▼
                              lamport += 1, event_id üret ─► transport ─► eş

eşten Clip ─► core: event_id görüldü mü? ── evet ─► at
                    │ hayır
                    ▼
              remote.lamport > son_yerel_lamport? (eşitse küçük device_id kazanır)
                    │ evet                             │ hayır ─► at (yerel öncelik)
                    ▼
              hash'i kaydet ─► clipboard adaptörü panoya yazar
```

## Pano adaptörleri

- **macOS:** `NSPasteboard.generalPasteboard`; `changeCount` 250 ms aralıkla okunur, değişince `public.utf8-plain-text` alınır. `org.nspasteboard.ConcealedType` veya `org.nspasteboard.TransientType` tipi varsa içerik okunmaz.
- **Windows:** `GetClipboardSequenceNumber` 250 ms aralıkla okunur (PoC'de mesaj penceresi gerektirmediği için `AddClipboardFormatListener` yerine); değişince `CF_UNICODETEXT` okunur. `ExcludeClipboardContentFromMonitorProcessing` biçimi varsa ya da `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard` = 0 ise içerik okunmaz.

## Bağlantı yaşam döngüsü

- Her iki cihaz da dinler ve eşi arar; aynı anda iki bağlantı kurulursa küçük `device_id`'nin başlattığı bağlantı kalır.
- 5 sn'de bir `Ping`; 15 sn yanıt yoksa kopuk sayılır, 1→30 sn üstel beklemeyle yeniden bağlanılır.
- Çevrimdışı kuyruk yok; yeniden bağlanınca pano durumu gönderilmez.

## Saklama

- Cihaz anahtarı: OS anahtarlığı (Keychain / Credential Manager); imzasız CLI'da sorun çıkarsa PoC için 0600 izinli dosya.
- Eş listesi: kullanıcı config dizininde `peers.json` (yalnız parmak izi, ad, son adres).

## Bilinmeyenler (Faz 0'da ölçülecek)

1. macOS 26+ programatik pano okumada gizlilik uyarısı gösteriyor mu, imzasız CLI'da nasıl davranıyor?
2. Windows Güvenlik Duvarı mDNS (UDP 5353) ve dinleme portunu ilk açılışta nasıl ele alıyor?
3. Mac uykudan uyanınca mDNS kaydı ve TCP bağlantısı ne kadar sürede toparlanıyor?
