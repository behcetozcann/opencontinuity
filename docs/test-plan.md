# Test planı — Faz 0

Durum işaretleri: `[ ]` yapılmadı, `[x]` geçti, `[!]` kaldı (not ile). Gerçek cihaz testi yapılmadan madde işaretlenmez.

## Otomatik (`cargo test --workspace`)

- [x] Codec: her mesaj tipi gidiş-dönüş; 5 MiB üstü çerçeve reddi; bozuk gövde reddi.
- [x] Metin: boş, Türkçe (`ğüşıöçĞÜŞİÖÇ`), emoji (birleşik ZWJ dahil), CRLF ve LF korunur, 4 MiB sınırı.
- [x] Core: iki sahte pano arasında 1.000 olay — döngü yok, son durum tutarlı.
- [x] Core: eşzamanlı kopyalama — yerel kopya gecikmiş uzak olayla ezilmez; eşitlikte belirleyici sonuç.
- [x] Core: aynı `event_id` iki kez uygulanmaz.
- [x] Pairing: iki uçta SAS eşit; farklı exporter farklı SAS.
- [x] Transport (localhost): eşleşmiş iki örnek bağlanır; eşleşmemiş sertifika iki yönde reddedilir. (`unpair` sonrası red: M5'te `oc run` ile)
- [ ] Heartbeat: eş yanıt vermeyince 15 sn içinde kopuk sayılır ve yeniden bağlanır.

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

## Bulgular

- **macOS yazma yarışı (2026-10-17):** Yazan uygulama `clearContents` + `setString` yapınca sayaç yalnız bir kez artıyor; araya giren okuma boş pano görürse o kopya kaçıyordu (1/9). `read_after_change` boş görünce 100 ms sonra yeniden okuyor; sonra 20/20.
- **Test gücü (2026-10-17):** Yankı bastırma devre dışı bırakılınca 3 core testi düşüyor (1.000 olay testi dahil); testler döngüyü gerçekten yakalıyor.
- **Windows sayaç (2026-10-17):** Tek kopyalamada `GetClipboardSequenceNumber` birden fazla artabiliyor (593→598); uygulamalar birkaç biçim yazıyor. 250 ms yoklama bunları tek olaya indiriyor.
- **Yeniden başlayan cihazın ilk kopyası reddediliyordu (2026-10-17):** Windows `oc run` yeniden başlayınca Lamport saati 0'dan başladı; ilk kopyası (lamport 1) Mac'in 7 dk önceki kopyasıyla (lamport 1) eşit sayılıp reddedildi. Kök neden: saatler bağlantıda eşitlenmiyordu. Çözüm: `Clock` mesajı (protokol v2) + core'da yeniden üretme testi. Gerçek cihazda doğrulandı: yeniden başlatma sonrası ilk kopya (lamport 2) uygulandı, 10/10.
- **Aynı metnin birden çok gönderilmesi (2026-10-17):** 744 baytlık metin 3 kez gitti; o sırada başka bir pano senkron programı açıktı. Teşhis kaydıyla (sahip süreç + biçimler) o program kapalıyken Antigravity'den 9 kopyada 9 olay, tekrar yok. Antigravity tek kopyada 7 biçim yazıyor (sayaç +8); 250 ms yoklama bunları tek olaya indirdi. Sync programı açıkken sahip adıyla doğrulama yapılmadı.
- **Hızlı yeniden bağlanma döngüsü (2026-10-17):** Eski sürümle karşılaşınca Mac 1 sn'de bir bağlanıp koptu; bekleme TLS sonrası sıfırlanıyordu. Artık eşten ilk geçerli mesaj (`Clock`) gelince sıfırlanıyor. Gerçek cihazda henüz yeniden üretilmedi.
- **Uyku ve saat (2026-10-17):** macOS'ta `Instant` uykuda ilerlemiyor (Mac kaydı uykuyu ~56 sn kısa gösterdi). Heartbeat yine de uyanıştan sonraki ilk turda bağlantıyı kopuk saydı; davranış doğru, ama kayıttaki süreler uyku boyunca duvar saati değil.
- **IP'siz eşleştirme sonrası ilk kopya (2026-10-17):** "Firma Adı, İl / İlçe ve Açık Adres" 40 bayt, hash ba018a43 iki uçta aynı.
