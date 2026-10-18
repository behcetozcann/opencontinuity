# Test planı — Faz 0 ve Faz 1

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
| F1 | Kurulum/ilk açılış: Gatekeeper / SmartScreen uyarısı (adımlar not edilir) | [ ] | [ ] |
| F2 | macOS yerel ağ izni ve pano gizlilik uyarısı çıkıyor mu? (sonuç not edilir) | [x] v0.1.2 (Developer ID): Yerel Ağ izni güncellemeden sonra tuttu, eş bulundu; pano uyarısı çıkmadı | — |
| F3 | Menü çubuğu / tepsi simgesi ve menüsü görünür; Dock'ta simge yok | [x] LSUIElement; Dock'a elle sabitlenmişse noktasız simge kalır (kullanıcı kaldırdı) | [x] |
| F4 | Faz 0 eşleşmesiyle açılınca kendiliğinden bağlanır | [ ] | [ ] |
| F5 | Uygulamadan eşleşmeyi kaldır → iki tarafta Eşleştir → aynı kod → eşleşir | [ ] | [ ] |
| F6 | Mac → Windows: Türkçe + emoji + çok satır, Not Defteri'nde görsel kontrol | [x] 115 bayt gönderildi | [x] Türkçe/çok satır/sekme doğru; bayrak 🇹🇷 Windows'ta çizilmiyor (yazı tipi), bayt olarak korunuyor (F7 dönüşünde aynı) |
| F7 | Windows → Mac metin | [x] Not Defteri'nden dönen satır ZWJ aile + bayrak dahil bayt bayt aynı, CRLF yok | [x] |
| F8 | Duraklat: gönderilmez, gelen uygulanmaz; Devam et sonrası ilk kopya gider | [x] duraklatmada yerel kopya gönderilmedi, gelen 3 kopya uygulanmadı; devamda ilk kopya gitti (lamport 8) | [x] devam sonrası gelen yapıştırıldı |
| F9 | Pencere kapatılınca uygulama tepside çalışmayı sürdürür; Çık ile kapanır | [x] pencere kapandı, süreç sürdü (Çık denenmedi) | [x] (Çık denenmedi) |
| F10 | Oturum kapat/aç (veya yeniden başlat) sonrası kendiliğinden başlar ve bağlanır | [ ] | [ ] |
| F11 | Uyku/uyanma ve Wi-Fi kapat/aç sonrası toparlanır | [x] kapak 12:10:28–12:14:09; uyanınca kopma hemen algılandı, ~31 sn sonra kendiliğinden bağlandı (Windows Wi-Fi de kapalıydı, iki olay örtüştü). Kopukken yapılan kopya bağlanınca gönderilmedi, tekrar kopyalamak gerekti | [x] Wi-Fi kapat/aç sonrası kendiliğinden bağlandı |
| F12 | İkinci kez açmak yeni örnek başlatmaz, mevcut pencereyi öne getirir | [x] `open` sonrası tek süreç, pencere ekranda | [x] mevcut pencere geldi |

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
- **Kopukken yapılan kopya kayboluyor (2026-10-18):** bağlantı yokken kopya `Eş bağlı değil; kopya gönderilmedi.` ile atılıyor, bağlanınca gönderilmiyor; kullanıcı Windows'ta 5–6 kez eski metni yapıştırdı. Bağlanınca son bekleyen kopyayı göndermek tasarım kararı — kullanıcıya soruldu.
- **Duraklatmada 3 kopya (2026-10-18):** Windows'tan 0,5 sn arayla 3 × 17 bayt geldi; Windows kaydı her birinin ayrı Chrome kopyası olduğunu gösterdi (sayaç +9, kullanıcı birkaç kez bastı). Hata değil.
