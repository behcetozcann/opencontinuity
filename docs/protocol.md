# Protokol v6 (taslak)

v6 (2026-10-18): çoklu cihaz — `Peers` mesajı (grup tanıtma). v5 (2026-10-18): dosya/klasör aktarımı (`FilesStart` + `FileChunk`), süren aktarımı bırakan `TransferCancel`. v4 (2026-10-18): `Clip` metin ya da PNG taşır (`payload`); görseller `ImageStart` + `ImageChunk` parçalarıyla gider. v3 (2026-10-17): `Hello`'ya `listen_port` ve `pairing` eklendi; eşleştirme modundaki uç yalnız eşleştirme modundaki uçla konuşur. v2: `Clock` mesajı eklendi. Sürüm `Hello` içinde karşılaştırılır; uyuşmazsa bağlantı açık bir hatayla kapanır.

## Taşıma

Karşılıklı kimlik doğrulamalı TLS 1.3 üzerinde tek TCP bağlantısı; varsayılan port **47800**. TLS sonrası iki uç da önce `Hello` gönderir; `device_id` karşı sertifikanın parmak izinin ilk 16 baytına eşit değilse bağlantı kapanır. Sertifikalar cihaz başına self-signed; güven, eşleştirmede doğrulanan SHA-256 parmak izine dayanır.

## Çerçeve

```text
┌──────────────────────┬────────────────────────────┐
│ uzunluk: u32 (BE)    │ gövde: postcard(Message)   │
└──────────────────────┴────────────────────────────┘
```

- Gövde en fazla **5 MiB**; aşan çerçeve bağlantıyı kapatır.
- Metin en fazla **4 MiB** (UTF-8 bayt).
- Dosya kopyası toplam en fazla **500 MiB**, en çok 5.000 girdi, yol ≤ 512 bayt. İçerik diskten okunup diske yazılarak akar; bellekte tutulmaz.
- Görsel en fazla **32 MiB** (PNG bayt), en çok **256 KiB**'lık parçalarla. Her parça ayrı çerçevedir; uzun aktarımda da her çerçeve eşi canlı gösterir (heartbeat kesilmez).

## Mesajlar

| Mesaj | Alanlar | Not |
|---|---|---|
| `Hello` | `version: u16`, `device_id: [u8; 16]`, `name: String`, `listen_port: u16`, `pairing: bool` | Bağlantıdaki ilk mesaj; sürüm ya da mod (`pairing`) uyuşmazsa bağlantı kapanır. `listen_port` karşı tarafın geri bağlanacağı port |
| `PairDecision` | `accepted: bool` | Yalnız eşleştirmede; iki uç da `true` göndermeden eş kaydedilmez |
| `Clock` | `lamport: u64` | `oc run` bağlantısında `Hello`'dan hemen sonra; alan taraf saatini `max(kendi, gelen)` yapar. Yeniden başlayan cihazın ilk kopyası böylece eski görünmez |
| `Clip` | `event_id: [u8; 16]`, `lamport: u64`, `payload: Text(String) \| Png(Vec<u8>)` | Pano olayı. `Png` PNG imzasıyla başlamalı. Gönderici görselleri bu mesajla değil parçalarla yollar |
| `ImageStart` | `event_id`, `lamport`, `total_bytes: u32` | Parçalı görselin başı; 1 ≤ `total_bytes` ≤ 32 MiB. Önceki görsel bitmeden yenisi gelirse bağlantı kapanır |
| `FilesStart` | `event_id`, `lamport`, `files: FileList` | Dosya aktarımının başı. `FileList.entries`: `{ path, size, is_dir }`; yol göreli, `/` ayraçlı, her adı `portable_name` ile aynı olmalı (Windows'ta geçersiz karakter, sondaki nokta/boşluk, `CON` gibi adlar yok), klasör içeriğinden önce gelir, büyük/küçük harf duyarsız tekrar yok. Gönderici klasör içeriğini ada göre sıralar; iki uç aynı listeyi üretir (yankı karşılaştırması buna dayanır) |
| `FileChunk` | `data: Vec<u8>` | Listedeki dosyaların içeriği sırayla; 1 B – 256 KiB. Fazla veri bağlantıyı kapatır |
| `TransferCancel` | — | Süren görsel/dosya aktarımı bırakıldı (yeni kopya geldi ya da kaynak dosya okunamadı/değişti); alan yarım kalanı siler. Kontrol mesajları (`Ping`/`Pong`) parçaların arasına girebilir |
| `ImageChunk` | `data: Vec<u8>` | 1 B – 256 KiB. Başlangıçsız parça ya da bildirilen boyutu aşan toplam bağlantıyı kapatır; tamamlanınca alan taraf tek `Clip` olayı olarak işler |
| `Ping` | `nonce: u64` | 5 sn'de bir |
| `Pong` | `nonce: u64` | |
| `Peers` | `peers: [{ name, fingerprint: [u8; 32], address: Option<String> }]` | Göndericinin güvendiği diğer cihazlar; senkron bağlantısında `Clock`'tan hemen sonra ve gruba yeni cihaz eklenince. ≤ 64 kayıt, ad ≤ 256 B, adres ≤ 64 B. Alan, kendisini ve kullanıcının kaldırdığı cihazları atlar; bilinmeyenleri ekler ve bağlı diğer eşlerine duyurur |

`device_id` sertifikanın parmak izinden türetilir (ilk 16 bayt), böylece taklit edilemez.

## Eşleştirme (SAS)

1. Taraflar, eşleşmemiş sertifikayı geçici olarak kabul eden bir TLS bağlantısı kurar.
2. Her iki uç `SHA-256("oc-sas-v1" ‖ min(fp_a, fp_b) ‖ max(fp_a, fp_b) ‖ tls_exporter("oc-sas", 32))` hesaplar; ilk 8 baytın `u64 % 1_000_000` değeri 6 haneli koddur.
3. Kullanıcı iki ekranda aynı kodu görüp iki uçta da onaylar; her uç kararını `PairDecision` ile bildirir. Biri reddederse hiçbir şey kaydedilmez.
4. Onaydan sonra karşı tarafın parmak izi `peers.json`'a yazılır; sonraki bağlantılarda yalnız gruptaki parmak izleri kabul edilir (0.1.7'nin `peer.json`'ı ilk açılışta taşınır).

## Grup (v6)

- Tam ağ: her cihaz gruptaki her cihazla ayrı TLS bağlantısı tutar (çift başına küçük parmak izli taraf bağlanır). Kopya bağlı bütün eşlere doğrudan gider; aktarma yok.
- Yeni cihaz gruptaki **herhangi bir** cihazla kod onayıyla eşleşir. Bağlantılardaki `Peers` alışverişiyle diğerleri onu, o da diğerlerini öğrenir. Güven geçişlidir: eşleşmiş bir cihazın tanıttığına güvenilir.
- Kaldırma yereldir ve kalıcıdır: kaldırılan parmak izi `peers.json`'da `removed` listesine girer, başka eş tanıtsa da eklenmez; aynı cihazla yeniden kod onayı kaydı siler.
- Pano sürümü `(lamport, gönderen)`; uzak olay ancak panodaki içerikten yeniyse uygulanır (eşitlikte küçük `device_id`). Üç ve daha çok cihazda eşzamanlı kopyalar da aynı içerikte birleşir.
