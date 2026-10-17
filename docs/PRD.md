# PRD — OpenContinuity

Ayrıntılı ürün planı ve kararlar: [`../OpenContinuity_Proje_Plani.md`](../OpenContinuity_Proje_Plani.md). Bu belge yalnızca o planın uygulanabilir özetidir; çelişkide plan geçerlidir.

## Sorun

Mac ve Windows arasında metin taşımak için bugün e-posta, mesajlaşma uygulaması ya da IP/port/sunucu/hesap isteyen araçlar gerekiyor.

## Hedef kullanıcı

Önce proje sahibi (kişisel + klinik kullanımı); repo baştan açık.

## Faz 0 (PoC) başarı ölçütü

- Aynı LAN'daki bir Mac ve bir Windows PC, IP girmeden birbirini bulur ve 6 haneli kodla bir kez eşleşir.
- Mac'te `Cmd+C` → Windows'ta `Ctrl+V` ve tersi çalışır; fare/klavye taşımak gerekmez.
- Türkçe, emoji, CRLF/LF, boş ve 4 MB'a kadar metin bozulmadan gider.
- Parola yöneticilerinin gizli/geçici işaretli kopyaları gönderilmez.
- Bağlantı kopması ve uyku/uyanma sonrası kendiliğinden toparlanır.
- Uzaktan gelen içerik geri yansımaz (döngü yok); az önceki yerel kopya, gecikmiş uzak veriyle ezilmez.

## Kapsam dışı (Faz 0 ve V0.1)

Görsel, zengin metin, dosya, pano geçmişi, çoklu cihaz, bulut/relay, mobil, fare/klavye paylaşımı, çevrimdışı kuyruk.

## Arayüz

Faz 0'da yalnız komut satırı aracı (`oc`). Tauri menü çubuğu / sistem tepsisi arayüzü Faz 1'de.
