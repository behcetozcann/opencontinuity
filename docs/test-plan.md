# Test planı — Faz 0, Faz 1 ve Faz 2

Durum işaretleri: `[ ]` yapılmadı, `[x]` geçti, `[!]` kaldı (not ile). Gerçek cihaz testi yapılmadan madde işaretlenmez.

## Otomatik (`cargo test --workspace`)

- [x] Codec: her mesaj tipi gidiş-dönüş; 5 MiB üstü çerçeve reddi; bozuk gövde reddi.
- [x] Metin: boş, Türkçe (`ğüşıöçĞÜŞİÖÇ`), emoji (birleşik ZWJ dahil), CRLF ve LF korunur, 4 MiB sınırı.
- [x] Core: iki sahte pano arasında 1.000 olay — döngü yok, son durum tutarlı.
- [x] Core: eşzamanlı kopyalama — yerel kopya gecikmiş uzak olayla ezilmez; eşitlikte belirleyici sonuç.
- [x] Core: aynı `event_id` iki kez uygulanmaz.
- [x] Pairing: iki uçta SAS eşit; farklı exporter farklı SAS.
- [x] Transport (localhost): eşleşmiş iki örnek bağlanır; eşleşmemiş sertifika iki yönde reddedilir; eşleştirme ve senkron modundaki uçlar oturum kuramaz (v3).
- [x] Engine (iki motor, sahte pano, localhost): eşleştirme + iki yönlü aktarım + yankı döngüsü yok; duraklatmada gönderim/uygulama yok ve devamdaki ilk kopya reddedilmez; `unpair` sonrası eski eş reddedilir ve yeniden eşleştirme çalışır; kod reddinde iki uçta da kayıt yok.
- [x] Görsel (v4): 10 MiB'lık görsel parçalanıp birleşir; başlangıçsız/iç içe/fazla parça ve PNG imzasız içerik reddedilir; çerçeve sınırını aşan görsel iki motor arasında gider, yankısı dönmez, bağlantı kopmaz; aynı baytlı metin ve görsel yankı sayılmaz.
- [x] Pano (Windows dönüşümü, her platformda): PNG → CF_DIB → PNG piksel piksel aynı; saydam piksel DIB'de beyaz; PNG uzunluğu sondaki dolguyu atar.
- [x] Dosya (v5): liste doğrulaması (`..`, mutlak yol, Windows'ta geçersiz ad, sırasız klasör, büyük/küçük harf tekrarı, sınır); klasör ağacı ada göre sıralı toplanır, çöp dosyalar atlanır; parçalar diske yazılır, fazla veri ve iptal yarım klasörü bırakmaz; budama yalnız tarihli klasörlere dokunur. İki motor arasında klasör + dosya bayt bayt gider, yankı dönmez; yeni kopya süren aktarımı bırakır, yarım klasör kalmaz; 500 MiB üstü `TooLarge`.
- [x] Grup (v6): üç sahte pano arasında 3.000 olay, ayrı kuyruklar ve karışık sırayla — döngü yok, üç cihaz aynı içerikte biter (eski kural burada ayrışıyordu). Üç motor: Linux yalnız Windows'la eşleşir, Mac onu tanıtmayla öğrenip bağlanır, kopyalar herkese gider, yankı yok; kapalı üye dönünce bekleyen kopyayı alır, diğerleri senkron sürer; kaldırılan üye başka eş tanıtsa da geri gelmez. `peer.json` → `peers.json` taşıma. Kabul eden taraf gruptaki her parmak izini tanır.
- [ ] Linux pano (CI, Xvfb): iki X istemcisi arasında metin, 3 MiB metin ve 5 MiB PNG (INCR, bayt bayt aynı), dosya listesi, sayaç; xclip ile karşılıklı okuma/yazma. İlk CI koşusu bekleniyor.
- [x] Heartbeat: susan eş zaman aşımıyla kopuk sayılır (testte kısaltılmış sürelerle: 600 ms; gerçek değer 15 sn).

## Gerçek cihaz (Mac ↔ Windows, aynı LAN)

| # | Senaryo | Mac | Windows |
|---|---|---|---|
| 1 | `oc clip-watch` pano değişikliğini görür (içerik yazmadan) | [x] 20/20 olay (düzeltme sonrası) | [x] 4/4 olay (Türkçe çok satırlı metin + parola) |
| 2 | macOS'ta pano okuma gizlilik uyarısı çıkıyor mu? (sonuç not edilir) | [!] Terminal'den: uyarı yok, `accessBehavior=always-allow`. Ayrı uygulama paketinde (Faz 1) yeniden ölçülecek | — |
| 3 | Parola yöneticisinden kopyalanan parola gönderilmez | [!] Concealed/Transient tipleri simülasyonla tespit edildi; gerçek parola yöneticisiyle denenmedi | [x] Chrome Parola Yöneticisi kopyası "gizli" algılandı |
| 4 | IP girmeden birbirini bulur | [x] mDNS ile buldu, kod 768 170 iki ekranda aynı | [x] Güvenlik Duvarı uyarısı çıkmadı |
| 5 | 6 haneli kod iki ekranda aynı; reddedilince eşleşme kaydedilmez | [x] kod 957 010 ve 768 170 iki ekranda aynı; red senaryosu yerelde iki örnekle | [x] |
| 6 | Mac `Cmd+C` → Windows `Ctrl+V` | [x] gönderim kaydı eşleşti | [x] alım kaydı eşleşti; Not Defteri'nde görsel kontrol bekleniyor |
| 7 | Windows `Ctrl+C` → Mac `Cmd+V` | [x] 10/10 alındı (Antigravity, Windows Terminal) | [x] |
| 8 | Türkçe + emoji + çok satırlı metin bozulmaz | [x] Windows→Mac "ve Diş Sağlığı Eryaman" hash iki uçta aynı (ce607f7b) | [!] Mac→Windows emoji/çok satır görsel kontrolü yapılmadı |
| 9 | Wi-Fi kapat/aç sonrası kendiliğinden toparlanır (süre not edilir) | [x] kopma 15–20 sn'de algılandı, elle işlem gerekmedi | [x] Windows Wi-Fi kapatıldı; açılış anı ölçülmedi |
| 10 | Uyku/uyanma sonrası toparlanır (süre not edilir) | [x] kapak 16:55:04–16:56:15 (pmset); uyanınca ilk heartbeat'te kopuk sayıldı, 1,1 sn'de yeniden bağlandı, sonraki kopya ulaştı | [x] |
| 11 | Yeniden başlatma sonrası tekrar eşleştirme istemez | [x] `oc run` yeniden başlatmaları (süreç, makine değil) | [x] süreç yeniden başlatma |
| 12 | Elle adres (`--addr`) ile eşleştirme çalışır | [x] dinleyen taraf | [x] bağlanan taraf |

## Gerçek cihaz — Faz 1 masaüstü uygulaması

Mac: `OpenContinuity.app` (.dmg, imzasız). Windows: `OpenContinuity_0.0.1_x64-setup.exe` (imzasız). İki tarafta da eski `oc run` kapalı, sync programı kapalı.

| # | Senaryo | Mac | Windows |
|---|---|---|---|
| F1 | Kurulum/ilk açılış: Gatekeeper / SmartScreen uyarısı (adımlar not edilir) | [x] kullanıcı: sorunsuz (2026-10-18, toplu onay) | [x] kullanıcı: sorunsuz (2026-10-18, toplu onay) |
| F2 | macOS yerel ağ izni ve pano gizlilik uyarısı çıkıyor mu? (sonuç not edilir) | [x] v0.1.2 (Developer ID): Yerel Ağ izni güncellemeden sonra tuttu, eş bulundu; pano uyarısı çıkmadı | — |
| F3 | Menü çubuğu / tepsi simgesi ve menüsü görünür; Dock'ta simge yok | [x] LSUIElement; Dock'a elle sabitlenmişse noktasız simge kalır (kullanıcı kaldırdı) | [x] |
| F4 | Faz 0 eşleşmesiyle açılınca kendiliğinden bağlanır | [ ] | [ ] |
| F5 | Uygulamadan eşleşmeyi kaldır → iki tarafta Eşleştir → aynı kod → eşleşir | [x] kullanıcı: sorunsuz (2026-10-18, toplu onay) | [x] kullanıcı: sorunsuz (2026-10-18, toplu onay) |
| F6 | Mac → Windows: Türkçe + emoji + çok satır, Not Defteri'nde görsel kontrol | [x] 115 bayt gönderildi | [x] Türkçe/çok satır/sekme doğru; bayrak 🇹🇷 Windows'ta çizilmiyor (yazı tipi), bayt olarak korunuyor (F7 dönüşünde aynı) |
| F7 | Windows → Mac metin | [x] Not Defteri'nden dönen satır ZWJ aile + bayrak dahil bayt bayt aynı, CRLF yok | [x] |
| F8 | Duraklat: gönderilmez, gelen uygulanmaz; Devam et sonrası ilk kopya gider | [x] duraklatmada yerel kopya gönderilmedi, gelen 3 kopya uygulanmadı; devamda ilk kopya gitti (lamport 8) | [x] devam sonrası gelen yapıştırıldı |
| F9 | Pencere kapatılınca uygulama tepside çalışmayı sürdürür; Çık ile kapanır | [x] pencere kapandı, süreç sürdü; Çık: kullanıcı sorunsuz (2026-10-18) | [x] Çık: kullanıcı sorunsuz (2026-10-18) |
| F10 | Oturum kapat/aç (veya yeniden başlat) sonrası kendiliğinden başlar ve bağlanır | [x] kullanıcı: sorunsuz (2026-10-18, toplu onay) | [x] 0.1.5'te başlamadı (aşağıda); 0.1.6+ ile kullanıcı defalarca yeniden başlattı, açılışta açılıyor (2026-10-18) |
| F11 | Uyku/uyanma ve Wi-Fi kapat/aç sonrası toparlanır | [x] kapak 12:10:28–12:14:09; uyanınca kopma hemen algılandı, ~31 sn sonra kendiliğinden bağlandı (Windows Wi-Fi de kapalıydı, iki olay örtüştü). Kopukken yapılan kopya bağlanınca gönderilmedi, tekrar kopyalamak gerekti | [x] Wi-Fi kapat/aç sonrası kendiliğinden bağlandı |
| F12 | İkinci kez açmak yeni örnek başlatmaz, mevcut pencereyi öne getirir | [x] `open` sonrası tek süreç, pencere ekranda | [x] mevcut pencere geldi |

## Gerçek cihaz — Faz 2 görsel panosu (v4)

İki tarafta da v4 (`oc run` ya da 0.1.3+) gerekir; v3 ile bağlanmaz.

| # | Senaryo | Mac | Windows |
|---|---|---|---|
| G0 | macOS adaptörü: PNG oku, TIFF→PNG çevir, yazılan PNG geri okununca aynı hash (`oc clip-watch` + `oc clip-set-image`) | [x] 2026-10-18: PNG 4142 B f8819d90; TIFF→PNG 5124 B; yazılan geri okununca f8819d90 | — |
| G1 | Mac ekran görüntüsü (⌘⇧⌃4) → Windows'ta Paint ve Word'e yapıştır; iki uçta hash aynı | [x] 0.1.4: 17464 B, hash af5f71c3 iki uçta aynı | [x] panoya PNG + CF_DIB yazıldı (Paint/Word ayrı denenmedi) |
| G2 | Windows ekran alıntısı (Win+Shift+S) → Mac'te Önizleme (Dosya → Panodan Yeni) ve Notlar'a yapıştır | [x] 0.1.3: 93171 B, hash a7200232 | [x] Ekran Alıntısı Aracı panoyu 0,3 sn sonra aynı içerikle yeniden yazıyor → 2 gönderim (zararsız). Alıntı Windows'ta kendiliğinden panoya girer; Mac ⌘⇧4 dosyaya kaydeder, panoya girmez |
| G3 | Tarayıcıda "Görseli kopyala" (Chrome/Edge/Safari) iki yönde | [ ] | [ ] |
| G4 | Saydam PNG: Mac'te saydamlık korunur; Windows Paint'te beyaz zemin, Word'de saydam | [ ] | [ ] |
| G5 | Büyük görsel (5K ekran görüntüsü, >5 MiB): bağlantı kopmaz, ardından metin de gider | [ ] | [ ] |
| G6 | Excel hücreleri / Word metni kopyala → metin gider, görsel gitmez | [ ] | [ ] |
| G7 | Alınan görselin yankısı geri gönderilmez (karşı tarafta "Alındı" sonrası "Gönderildi" yok) | [ ] | [ ] |
| G8 | ~~Finder / Gezgin'de dosya kopyala → dosya adı metni gider~~ 0.1.4'ten beri dosya aktarılıyor, bkz. D1–D8 | [ ] | [ ] |

## Gerçek cihaz — Faz 2 dosya aktarımı (v5)

2026-10-18: kullanıcı 0.1.4'te kalan G ve D testlerini "testler tamam" diye toplu bildirdi; madde madde sonuç ve kayıt alınmadı, bu yüzden aşağıda ayrı ayrı işaretlenmedi. Bir sonraki sorun bildiriminde ilgili madde kayıtla doğrulanır.

| # | Senaryo | Mac | Windows |
|---|---|---|---|
| D0 | macOS adaptörü: Finder kopyası okunur, yazılan 2 öğe yazan süreç kapandıktan sonra da 2 kalır (`oc clip-set-files` + JXA sayımı) | [x] 2026-10-18 (ilk sürümde 1 kalıyordu, düzeltildi) | — |
| D1 | Tek PDF: Mac Finder'da Cmd+C → Windows masaüstünde Ctrl+V | [ ] | [ ] |
| D2 | Gezgin'de Excel/Word dosyası Ctrl+C → Mac Finder'da Cmd+V | [x] 0.1.4: 1 öğe 99988 B alındı, kullanıcı "çalışıyor" dedi | [x] |
| D3 | İç içe klasör (boş alt klasör dahil) iki yönde; yapı ve içerik aynı | [ ] | [ ] |
| D4 | Türkçe/emoji adlı dosya; Mac'te `:` içeren ad Windows'ta `_` ile gelir | [ ] | [ ] |
| D5 | 200 MB+ dosya: "gönderiliyor/alınıyor" görünür, bu sırada metin kopyalanırsa aktarım bırakılır ve metin gider | [ ] | [ ] |
| D6 | 500 MB üstü: "gönderilmedi · 500 MB sınırını aşıyor" | [ ] | [ ] |
| D7 | "Göster" gelen klasörü açar; Ayarlar → Gelen dosyalar → Klasörü aç | [ ] | [ ] |
| D8 | Alınan dosyalar yankı olarak geri gönderilmez | [ ] | [ ] |

## Gerçek cihaz — Faz 2 çoklu cihaz (v6, 0.1.8)

İki cihaz da 0.1.8'e güncellenmeden bağlanmaz (protokol v6). Üçüncü cihaz yoksa M1, M2, M6 iki cihazla da yapılabilir.

| # | Senaryo | Sonuç |
|---|---|---|
| M1 | 0.1.7'den güncelleme: eşleşme korunur (`peer.json` taşınır), kendiliğinden bağlanır | [ ] |
| M2 | Ana ekran ve tepsi tek eşte eskisi gibi ("Bağlı · Windows-PC") | [ ] |
| M3 | Üçüncü cihaz: gruptaki birinde Ayarlar › Cihaz ekle, yenide Eşleştirmeye başla → kod → diğer cihaz yeniyi kendiliğinden tanır ve bağlanır | [ ] |
| M4 | Üç cihazda her birinden kopya diğer ikisine gider; "Son aktarımlar"da kaynağın adı görünür | [ ] |
| M5 | Bir cihaz kapalıyken diğer ikisi senkron sürer; ana ekranda "3 cihazdan 2 tanesi bağlı" | [ ] |
| M6 | Ayarlar › Cihazlar › Bu cihazı kaldır: o cihazla paylaşım durur, diğerleriyle sürer; uygulamayı yeniden açınca geri gelmez | [ ] |
| M7 | Cihaz eklerken (eşleştirme açıkken) gruptaki mevcut bağlantılar kopmaz (0.1.9) | [x] 2026-10-19 kullanıcı gerçek cihazda doğruladı |

## Gerçek cihaz — Linux (Mint, Cinnamon X11, 0.1.8)

Önce `oc info` ile "Linux oturumu: x11" görülmeli. Pano teşhisi: `oc clip-watch`.

| # | Senaryo | Sonuç |
|---|---|---|
| L1 | `.deb` çift tıkla kurulur; menüden açılır; tepside (sağ altta) simge ve menüsü görünür | [x] 2026-10-19 DentOs (Mint 22.3 Cinnamon, MacBook Air), 0.1.8 test-5720307 |
| L2 | Gruba katılım: Linux'ta Eşleştirmeye başla, Mac'te Ayarlar › Cihaz ekle → kod → Windows da Linux'u kendiliğinden tanır | [x] 2026-10-19 — ilk denemede Mac'te "Kodlar aynı"dan 0,2 sn sonra Vazgeç'e düşüp iptal oldu (düzeltildi, yeniden denenecek); Linux önce onaylayınca geçti, Windows'a 5 sn'de bağlandı |
| L3 | Metin: Xed / Firefox'ta kopyala ↔ Mac ve Windows'ta yapıştır (Türkçe + emoji, çok satır) | [x] 2026-10-19 üç cihaz, iki yön |
| L4 | Görsel: Mint ekran görüntüsü "panoya kopyala" → Mac/Windows; Mac'ten gelen görsel GIMP/LibreOffice'e yapıştırılır | [x] 2026-10-19 LibreOffice'e yapıştı; Nemo/masaüstü görsel verisini dosyaya çevirmez (beklenen) |
| L5 | Dosya: Nemo'da Ctrl+C → Mac Finder / Windows Gezgin'e yapıştır; tersi Nemo'ya Ctrl+V ile gelir | [x] 2026-10-19 Linux → Mac 3 öğe 17 MB, Mac Finder → Linux 3 öğe; Windows'a da |
| L6 | Oturumu kapat/aç: kendiliğinden başlar (`~/.config/autostart/OpenContinuity.desktop`) ve bağlanır | [x] 2026-10-19 5 sn'de iki eşe bağlandı; kopukken yapılan Mac kopyası bağlanınca gitti |
| L7 | Parola yöneticisinden (KeePassXC) kopya gönderilmez | [x] 2026-10-19 `gizli/geçici, atlandı` (x-kde-passwordManagerHint); Mac panosunda parola yok |

## Bulgular

- **macOS yazma yarışı (2026-10-17):** Yazan uygulama `clearContents` + `setString` yapınca sayaç yalnız bir kez artıyor; araya giren okuma boş pano görürse o kopya kaçıyordu (1/9). `read_after_change` boş görünce 100 ms sonra yeniden okuyor; sonra 20/20.
- **Test gücü (2026-10-17):** Yankı bastırma devre dışı bırakılınca 3 core testi düşüyor (1.000 olay testi dahil); testler döngüyü gerçekten yakalıyor.
- **Windows sayaç (2026-10-17):** Tek kopyalamada `GetClipboardSequenceNumber` birden fazla artabiliyor (593→598); uygulamalar birkaç biçim yazıyor. 250 ms yoklama bunları tek olaya indiriyor.
- **Yeniden başlayan cihazın ilk kopyası reddediliyordu (2026-10-17):** Windows `oc run` yeniden başlayınca Lamport saati 0'dan başladı; ilk kopyası (lamport 1) Mac'in 7 dk önceki kopyasıyla (lamport 1) eşit sayılıp reddedildi. Kök neden: saatler bağlantıda eşitlenmiyordu. Çözüm: `Clock` mesajı (protokol v2) + core'da yeniden üretme testi. Gerçek cihazda doğrulandı: yeniden başlatma sonrası ilk kopya (lamport 2) uygulandı, 10/10.
- **Aynı metnin birden çok gönderilmesi (2026-10-17):** 744 baytlık metin 3 kez gitti; o sırada başka bir pano senkron programı açıktı. Teşhis kaydıyla (sahip süreç + biçimler) o program kapalıyken Antigravity'den 9 kopyada 9 olay, tekrar yok. Antigravity tek kopyada 7 biçim yazıyor (sayaç +8); 250 ms yoklama bunları tek olaya indirdi. Sync programı açıkken sahip adıyla doğrulama yapılmadı.
- **Hızlı yeniden bağlanma döngüsü (2026-10-17):** Eski sürümle karşılaşınca Mac 1 sn'de bir bağlanıp koptu; bekleme TLS sonrası sıfırlanıyordu. Artık eşten ilk geçerli mesaj (`Clock`) gelince sıfırlanıyor. Gerçek cihazda henüz yeniden üretilmedi.
- **Uyku ve saat (2026-10-17):** macOS'ta `Instant` uykuda ilerlemiyor (Mac kaydı uykuyu ~56 sn kısa gösterdi). Heartbeat yine de uyanıştan sonraki ilk turda bağlantıyı kopuk saydı; davranış doğru, ama kayıttaki süreler uyku boyunca duvar saati değil.
- **Duraklatma sonrası ilk kopya reddi (2026-10-17, otomatik test):** Duraklatmada gelen kopya uygulanmıyor ama Lamport saati de ilerlemiyordu; devamdaki ilk yerel kopya eşin son kopyasıyla aynı lamport'u alıp eşitlikte cihaz kimliği sırasına göre ~%50 reddediliyordu. Engine testi 8 koşuda kararsızdı; düzeltmeyle (duraklatmada da `observe_peer_clock`) 12/12.
- **Windows'ta `WebView2Loader.dll bulunamadı` (2026-10-17, gerçek cihaz):** gnu hedefiyle derlenen exe loader DLL'ini dinamik bağlıyor (`webview2-com-sys`: msvc dışı hedefte `link(name = "WebView2Loader.dll")`). Installer DLL'i kopyalasa da kullanıcıda açılışta bulunamadı (neden Windows tarafında doğrulanmadı). Masaüstü uygulaması artık `x86_64-pc-windows-msvc` + `cargo-xwin` ile derleniyor; loader statik bağlanıyor, exe'nin DLL bağımlılığı yok.
- **Mac'te uygulama "açılmıyor" (2026-10-17):** Eş kayıtlıyken pencere gizli başlıyordu, simge menü çubuğunda (çentik arkasında) kalınca kullanıcı uygulamayı açılmamış sandı. Elle açılışta pencere artık hep görünür; yalnız `--autostart` ile (girişte) gizli başlar.
- **Port 47800 dolu (2026-10-17):** Faz 0 testinden kalan arka plan `oc run` portu tutuyordu. Hata bandında "Yeniden dene" eklendi; port doluyken de kayıtlı eş gösteriliyor.
- **IP'siz eşleştirme sonrası ilk kopya (2026-10-17):** "Firma Adı, İl / İlçe ve Açık Adres" 40 bayt, hash ba018a43 iki uçta aynı.
- **Windows, Mac'in uykusunu geç fark etti (2026-10-18):** Mac kapağı 12:10:28'de kapandı, 12:14:09'da açıldı; Windows `eş 15 sn yanıt vermedi`'yi ancak 12:14:27'de yazdı (beklenen ~12:10:45). Arada Windows'tan kopya olsaydı "Gönderildi" görünüp kaybolurdu. Heartbeat kodu (5 sn ping, 15 sn `last_seen`) doğru görünüyor; aynı testte Windows Wi-Fi'si de kapatılıp açıldığı için olaylar örtüştü. Sıradaki: yalnız kapak kapat/aç, Windows'a dokunmadan, Windows `oc.log` ile yeniden üret. Not: macOS'ta kayıt zamanı (`Instant`) uykuda durur; duvar saatini `pmset -g log` + süreç başlangıcıyla eşledim, kayda duvar saati eklemek teşhisi kolaylaştırır.
- **Kopukken yapılan kopya kayboluyor (2026-10-18):** bağlantı yokken kopya `Eş bağlı değil; kopya gönderilmedi.` ile atılıyor, bağlanınca gönderilmiyor; kullanıcı Windows'ta 5–6 kez eski metni yapıştırdı. Bağlanınca son bekleyen kopyayı göndermek tasarım kararı — kullanıcıya soruldu. **Çözüm (0.1.5, gerçek cihazda denenmedi):** kopukken yapılan son kopya saklanır, 2 dk içinde bağlanılırsa kopyalandığı andaki Lamport değeriyle gönderilir (eş o arada kendisi kopyaladıysa iki taraf aynı sonuca varır); daha eskiyse bırakılır. Testler: `copy_made_while_disconnected_is_sent_on_reconnect`, `stale_offline_copy_is_not_sent`.
- **Duraklatmada 3 kopya (2026-10-18):** Windows'tan 0,5 sn arayla 3 × 17 bayt geldi; Windows kaydı her birinin ayrı Chrome kopyası olduğunu gösterdi (sayaç +9, kullanıcı birkaç kez bastı). Hata değil.
- **Windows'ta animasyonlar duruyordu (2026-10-18):** "Animasyon efektleri" kapalıydı → `prefers-reduced-motion`. 0.1.5'te hareket azaltmada durum animasyonları kalıyor; kullanıcı Windows'ta "efektler çalışıyor" dedi (gerçek cihaz onaylı).
- **Windows yeniden başlayınca uygulama açılmadı (2026-10-18, 0.1.5):** Kayıt HKCU'da değil HKLM `Run`'da: `\\?\C:\Users\User\AppData\Local\OpenContinuity\opencontinuity.exe --autostart` (`auto-launch` varsayılan kipi yetki varsa HKLM'ye yazıyor; yol `canonicalize`'dan önekli ve tırnaksız). İlk teşhisim ("kayıt silinmiş") yanlıştı — yalnız HKCU'ya bakmıştım. Muhtemel neden önekli yol; HKLM `StartupApproved` ölçülmedi. Düzeltme (0.1.6): kayıt HKCU'ya, öneksiz ve tırnaklı yazılır, her açılışta yenilenir; yalnız Ayarlar'dan kapatma `autostart-disabled` ile hatırlanır; eski HKLM kaydı da açarsa tek örnek pencereyi göstermez. Gerçek cihazda onaylı (kullanıcı, 2026-10-18).
