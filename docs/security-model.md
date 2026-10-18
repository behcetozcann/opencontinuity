# Güvenlik modeli — Faz 0

## Korunan varlıklar

1. Pano içeriği (parola, OTP, hasta verisi olabilir).
2. Cihaz özel anahtarı.
3. Eşleşme listesi (kime güvendiğimiz).

## Tehditler ve önlemler

| Tehdit | Önlem |
|---|---|
| Aynı ağda pasif dinleme | Tüm trafik TLS 1.3; düz metin port yok |
| Eşleştirmede araya girme (MITM) | 6 haneli SAS, TLS exporter'a bağlı; iki uçta kullanıcı onayı |
| Eşleşmemiş cihazın bağlanıp pano okuması/yazması | Eşleşme sonrası parmak izi sabitleme; bilinmeyen sertifika el sıkışmada reddedilir |
| mDNS'te sahte cihaz yayını | Keşif yalnızca adres verir; güven parmak izine dayanır, TXT kaydına değil |
| Kötü niyetli dosya adı ile hedef dışına yazma | Yollar göreli, `..`/mutlak/boş bileşen yok, her ad taşınabilir ad olmalı; klasör içeriğinden önce bildirilmeli; dosyalar yalnız `İndirilenler/OpenContinuity/.partial-<olay>` altında oluşturulur, sembolik bağ izlenmez. Toplam ≤ 500 MiB, ≤ 5.000 girdi |
| Kötü niyetli dev çerçeve / bellek tüketme | Çerçeve ≤ 5 MiB, metin ≤ 4 MiB, görsel ≤ 32 MiB (parçalı; toplam `ImageStart`'ta doğrulanmadan ayrılmaz), PNG imzası zorunlu; uzunluk okunmadan bellek ayrılmaz. Windows'ta DIB dönüşümü 50 MP üstünde yapılmaz |
| Geçersiz UTF-8 / bozuk mesaj | Çözme hatası bağlantıyı kapatır, panoya hiçbir şey yazılmaz |
| Log veya test kaydına pano içeriği sızması | İçerik asla loglanmaz; yalnız uzunluk + hash'in ilk 8 hex karakteri |
| Parola yöneticisi kopyalarının gönderilmesi | macOS Concealed/Transient tipleri, Windows Exclude/CanIncludeInClipboardHistory biçimleri varsa gönderilmez |
| Eşleşme kaldırıldıktan sonra erişim | `unpair` parmak izini siler; mevcut bağlantı kapatılır |
| Çalınan cihaz anahtarı | OS anahtarlığında saklanır (PoC yedeği: 0600 dosya) |
| İnternete açılma | Varsayılan olarak yalnız yerel ağ arayüzleri; UPnP/port yönlendirme yok |

## Kapsam dışı (Faz 0)

- Kötü amaçlı yazılımın yerel makinede panoyu okuması (OS seviyesinde, bizim kontrolümüzde değil).
- Eşleştirme koduna fiziksel olarak bakan kişi (omuz sörfü).
- Kod imzalama ve güncelleme zinciri güvenliği (Faz 1).

## Kurallar

- Kriptografi sıfırdan yazılmaz: rustls (ring), rcgen, sha2.
- Önemli güvenlik değişiklikleri insan incelemesinden geçer.
