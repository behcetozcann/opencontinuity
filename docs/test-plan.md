# Test planı — Faz 0

Durum işaretleri: `[ ]` yapılmadı, `[x]` geçti, `[!]` kaldı (not ile). Gerçek cihaz testi yapılmadan madde işaretlenmez.

## Otomatik (`cargo test --workspace`)

- [x] Codec: her mesaj tipi gidiş-dönüş; 5 MiB üstü çerçeve reddi; bozuk gövde reddi.
- [x] Metin: boş, Türkçe (`ğüşıöçĞÜŞİÖÇ`), emoji (birleşik ZWJ dahil), CRLF ve LF korunur, 4 MiB sınırı.
- [x] Core: iki sahte pano arasında 1.000 olay — döngü yok, son durum tutarlı.
- [x] Core: eşzamanlı kopyalama — yerel kopya gecikmiş uzak olayla ezilmez; eşitlikte belirleyici sonuç.
- [x] Core: aynı `event_id` iki kez uygulanmaz.
- [ ] Pairing: iki uçta SAS eşit; farklı exporter farklı SAS.
- [ ] Transport (localhost): eşleşmiş iki örnek bağlanır; eşleşmemiş sertifika reddedilir; `unpair` sonrası reddedilir.
- [ ] Heartbeat: eş yanıt vermeyince 15 sn içinde kopuk sayılır ve yeniden bağlanır.

## Gerçek cihaz (Mac ↔ Windows, aynı LAN)

| # | Senaryo | Mac | Windows |
|---|---|---|---|
| 1 | `oc clip-watch` pano değişikliğini görür (içerik yazmadan) | [x] 20/20 olay (düzeltme sonrası) | [ ] |
| 2 | macOS'ta pano okuma gizlilik uyarısı çıkıyor mu? (sonuç not edilir) | [!] Terminal'den: uyarı yok, `accessBehavior=always-allow`. Ayrı uygulama paketinde (Faz 1) yeniden ölçülecek | — |
| 3 | Parola yöneticisinden kopyalanan parola gönderilmez | [!] Concealed/Transient tipleri simülasyonla tespit edildi; gerçek parola yöneticisiyle denenmedi | [ ] |
| 4 | IP girmeden birbirini bulur | [ ] | [ ] |
| 5 | 6 haneli kod iki ekranda aynı; reddedilince eşleşme kaydedilmez | [ ] | [ ] |
| 6 | Mac `Cmd+C` → Windows `Ctrl+V` | [ ] | [ ] |
| 7 | Windows `Ctrl+C` → Mac `Cmd+V` | [ ] | [ ] |
| 8 | Türkçe + emoji + çok satırlı metin bozulmaz | [x] yerel gidiş-dönüş (eşler arası değil) | [ ] |
| 9 | Wi-Fi kapat/aç sonrası kendiliğinden toparlanır (süre not edilir) | [ ] | [ ] |
| 10 | Uyku/uyanma sonrası toparlanır (süre not edilir) | [ ] | [ ] |
| 11 | Yeniden başlatma sonrası tekrar eşleştirme istemez | [ ] | [ ] |
| 12 | Elle adres (`--addr`) ile eşleştirme çalışır | [ ] | [ ] |

## Bulgular

- **macOS yazma yarışı (2026-10-17):** Yazan uygulama `clearContents` + `setString` yapınca sayaç yalnız bir kez artıyor; araya giren okuma boş pano görürse o kopya kaçıyordu (1/9). `read_after_change` boş görünce 100 ms sonra yeniden okuyor; sonra 20/20.
- **Test gücü (2026-10-17):** Yankı bastırma devre dışı bırakılınca 3 core testi düşüyor (1.000 olay testi dahil); testler döngüyü gerçekten yakalıyor.
