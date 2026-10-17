# Test planı — Faz 0

Durum işaretleri: `[ ]` yapılmadı, `[x]` geçti, `[!]` kaldı (not ile). Gerçek cihaz testi yapılmadan madde işaretlenmez.

## Otomatik (`cargo test --workspace`)

- [ ] Codec: her mesaj tipi gidiş-dönüş; 5 MiB üstü çerçeve reddi; bozuk gövde reddi.
- [ ] Metin: boş, Türkçe (`ğüşıöçĞÜŞİÖÇ`), emoji (birleşik ZWJ dahil), CRLF ve LF korunur, 4 MiB sınırı.
- [ ] Core: iki sahte pano arasında 1.000 olay — döngü yok, son durum tutarlı.
- [ ] Core: eşzamanlı kopyalama — yerel kopya gecikmiş uzak olayla ezilmez; eşitlikte belirleyici sonuç.
- [ ] Core: aynı `event_id` iki kez uygulanmaz.
- [ ] Pairing: iki uçta SAS eşit; farklı exporter farklı SAS.
- [ ] Transport (localhost): eşleşmiş iki örnek bağlanır; eşleşmemiş sertifika reddedilir; `unpair` sonrası reddedilir.
- [ ] Heartbeat: eş yanıt vermeyince 15 sn içinde kopuk sayılır ve yeniden bağlanır.

## Gerçek cihaz (Mac ↔ Windows, aynı LAN)

| # | Senaryo | Mac | Windows |
|---|---|---|---|
| 1 | `oc clip-watch` pano değişikliğini görür (içerik yazmadan) | [ ] | [ ] |
| 2 | macOS'ta pano okuma gizlilik uyarısı çıkıyor mu? (sonuç not edilir) | [ ] | — |
| 3 | Parola yöneticisinden kopyalanan parola gönderilmez | [ ] | [ ] |
| 4 | IP girmeden birbirini bulur | [ ] | [ ] |
| 5 | 6 haneli kod iki ekranda aynı; reddedilince eşleşme kaydedilmez | [ ] | [ ] |
| 6 | Mac `Cmd+C` → Windows `Ctrl+V` | [ ] | [ ] |
| 7 | Windows `Ctrl+C` → Mac `Cmd+V` | [ ] | [ ] |
| 8 | Türkçe + emoji + çok satırlı metin bozulmaz | [ ] | [ ] |
| 9 | Wi-Fi kapat/aç sonrası kendiliğinden toparlanır (süre not edilir) | [ ] | [ ] |
| 10 | Uyku/uyanma sonrası toparlanır (süre not edilir) | [ ] | [ ] |
| 11 | Yeniden başlatma sonrası tekrar eşleştirme istemez | [ ] | [ ] |
| 12 | Elle adres (`--addr`) ile eşleştirme çalışır | [ ] | [ ] |
