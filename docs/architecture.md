# Mimari (EDD) — Faz 0 / Faz 1

## Bileşenler

| Crate | Görev |
|---|---|
| `protocol` | Sürümlü mesaj tipleri ve çerçeve codec'i ([protocol.md](protocol.md)) |
| `core` | Senkron durum makinesi: Lamport sayacı, döngü/tekrar önleme, çakışma |
| `clipboard` | Ortak `Clipboard` arayüzü + macOS (NSPasteboard) ve Windows (Win32) adaptörleri |
| `pairing` | Cihaz kimliği (self-signed P-256), eş listesi, 6 haneli SAS |
| `transport` | Karşılıklı TLS 1.3 (rustls, ring), parmak izi sabitleme, heartbeat |
| `discovery` | mDNS / DNS-SD yayını ve taraması (`_opencontinuity._tcp.local.`) |
| `engine` | Senkron motoru: boşta → eşleştirme → senkron döngüsü durum makinesi; dışarıya `Event`/`Snapshot`, içeriye `Command` (Faz 1) |
| `apps/cli` | `oc` binary'si; motoru çalıştırıp olayları yazdıran ince kabuk, teşhis aracı |
| `apps/desktop` | Tauri 2 uygulaması: menü çubuğu/tepsi, pencere arayüzü (`ui/`, derleme adımsız HTML/JS), girişte başlatma, tek örnek, teşhis kaydı `oc.log` |

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

- **macOS:** `NSPasteboard.generalPasteboard`; `changeCount` 250 ms aralıkla okunur, değişince `public.utf8-plain-text` alınır; metin yoksa (ya da boşsa) `public.png`, o da yoksa `public.tiff` okunup `NSBitmapImageRep` ile PNG'ye çevrilir. Uzak görsel `public.png` olarak yazılır. `org.nspasteboard.ConcealedType` veya `org.nspasteboard.TransientType` tipi varsa içerik okunmaz.
- **Windows:** `GetClipboardSequenceNumber` 250 ms aralıkla okunur (PoC'de mesaj penceresi gerektirmediği için `AddClipboardFormatListener` yerine); değişince `CF_UNICODETEXT`, yoksa kayıtlı `PNG` biçimi (uzunluk IEND'e göre kırpılır; pano belleği yuvarlanmış boyutla döner), o da yoksa `CF_DIB` okunup PNG'ye çevrilir. Pano yalnız baytlar kopyalanırken açık tutulur. Uzak görsel `PNG` + `CF_DIB` (32 bit, saydamlık beyaza karıştırılmış) olarak yazılır. `ExcludeClipboardContentFromMonitorProcessing` biçimi varsa ya da `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard` = 0 ise içerik okunmaz.
- **Dosyalar:** macOS'ta her pano öğesinin `public.file-url`'si (Finder dosya referans URL'si `filePathURL` ile yola çevrilir); yazarken her dosya, URL'si hemen yazılmış ayrı `NSPasteboardItem` (NSURL nesnesi yazılınca ilki dışındakiler tembel veri oluyor, yazan süreç kapanınca kayboluyordu). Windows'ta `CF_HDROP` (`DragQueryFileW`); yazarken `DROPFILES` + `Preferred DropEffect` = kopyala. Dosya kopyası metinden önce gelir (Finder adı metin, simgeyi TIFF olarak da koyar).
- **Gelen dosyalar:** `İndirilenler/OpenContinuity/.partial-<olay>` altına yazılır; kabul edilince tarihli klasöre (`2026-10-18 13.05.12`) taşınır ve panoya o yollar konur, yapıştırınca Finder/Gezgin hedefe kopyalar. Yalnız tarih adlı klasörler budanır, en yeni 10'u kalır; yarım aktarımlar senkron başlarken silinir. Sembolik bağlar izlenmez; `.DS_Store`, `Thumbs.db`, `desktop.ini` atlanır.
- **Yazma görevi:** Bağlantının yazma ucu ayrı görevdedir; görsel/dosya parça parça gider, her parçadan önce yeni iş var mı bakılır. Ping/pong araya girer; yeni pano olayı süren aktarımı `TransferCancel` ile bırakır (en son kopya geçerli). Döngü aktarım sürerken komutlara ve heartbeat'e yanıt vermeye devam eder.
- **Metin ve görsel birlikteyse metin gider:** Office/Excel kopyaları metnin görüntüsünü de koyar; kullanıcının beklediği metindir.
- Görsel yankısı metindeki gibi hash'le bastırılır; hash türü de içerir (aynı baytlı metin ve görsel ayrı sayılır). Yankı ancak yazılan PNG baytları aynen geri okunursa tutar: iki adaptör de kendi yazdığı PNG'yi önce okur.

## Bağlantı yaşam döngüsü

- Her iki cihaz da dinler; yalnız parmak izi küçük olan taraf bağlanır (çift bağlantı yarışı yok). Geri bağlanma adresi, eşin `Hello` ile bildirdiği dinleme portunu kullanır.
- Eşleştirme modundaki uç yalnız eşleştirme modundaki uçla oturum kurar (`Hello.pairing`); eşleşmesi kaldırılmış cihazın eski eşi eşleştirme akışını bozamaz.
- Duraklatmada bağlantı açık kalır; yerel kopyalar gönderilmez, gelenler uygulanmaz ama gelen olayın Lamport değeri gözlenir.
- 5 sn'de bir `Ping`; 15 sn hiçbir çerçeve gelmezse kopuk sayılır (okuma görevi her çerçevede `last_seen`'i günceller, görsel parçaları dahil), 1→30 sn üstel beklemeyle yeniden bağlanılır.
- Çevrimdışı kuyruk yok; yeniden bağlanınca pano durumu gönderilmez, yalnız Lamport saatleri `Clock` ile eşitlenir (yeniden başlayan cihazın ilk kopyası reddedilmesin diye; 2026-10-17 gerçek cihaz testinde bulundu).

## Saklama

- Cihaz anahtarı: PoC'de config dizininde 0600 izinli `identity.json`. İmzasız ve her derlemede değişen binary Keychain'de her seferinde onay isteyeceği için OS anahtarlığına imzalı uygulamayla geçilir; Faz 1 test sürümleri imzasız olduğu için dosyada kaldı.
- Eşler: kullanıcı config dizininde `peers.json` (her eş için ad, parmak izi, son adres; kullanıcının kaldırdıkları `removed`). Config dizini macOS'ta `~/Library/Application Support/org.OpenContinuity.OpenContinuity`, Windows'ta `%APPDATA%\OpenContinuity\OpenContinuity\config`.

## Bilinmeyenler (Faz 0'da ölçülecek)

1. macOS 26+ programatik pano okumada gizlilik uyarısı gösteriyor mu, imzasız CLI'da nasıl davranıyor?
2. Windows Güvenlik Duvarı mDNS (UDP 5353) ve dinleme portunu ilk açılışta nasıl ele alıyor?
3. Mac uykudan uyanınca mDNS kaydı ve TCP bağlantısı ne kadar sürede toparlanıyor?
