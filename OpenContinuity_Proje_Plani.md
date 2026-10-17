# OpenContinuity — Açık Kaynaklı Evrensel Cihaz Sürekliliği

> **Durum:** Fikir / planlama — geliştirme henüz başlamadı.  
> **Hazırlanma:** 17 Ekim 2026  
> **Çalışma adı:** OpenContinuity (marka, domain, GitHub organizasyon adı ve benzer projeler kontrol edilmedi).  
> **Vizyon:** Apple Universal Clipboard + AirDrop benzeri deneyimi Windows, macOS, Linux, Android ve iOS arasında; açık kaynaklı, güvenli ve kurulumu kolay şekilde sunmak.

## 1. Ürünün bir cümlelik tanımı

**İki cihazı aç, bir kez eşleştir; birinde kopyaladığını diğerinde normal yapıştır.**

Hedef yalnızca pano paylaşımı değil, uzun vadede cihazlar arası açık bir süreklilik ekosistemi kurmak. **İlk sürümde kapsamı bilinçli olarak dar tut:** Windows ↔ macOS arasında sorunsuz, iki yönlü **metin** kopyala/yapıştır; ardından görseller.

## 2. Çözdüğümüz sorun

Bugünkü uygulamalar çoğunlukla kullanıcıdan IP, port, sunucu kurulumu, hesap, izin, fare/klavye paylaşımı veya karmaşık ayarlar bekliyor. Bizim ilkelerimiz:

- **Zero-config:** Aynı yerel ağdaki cihazlar otomatik keşfedilsin.
- **Tek seferlik onay:** Eşleştir ve unut; yeniden başlatınca da çalışsın.
- **Gerçek pano:** Mac'te `Cmd+C` → Windows'ta `Ctrl+V`; Windows'ta `Ctrl+C` → Mac'te `Cmd+V`.
- **Bağımsız giriş aygıtları:** Fare veya klavyeyi diğer bilgisayara geçirmek gerekmemeli.
- **Önce yerel:** Hesap, bulut ya da Linux sunucusu zorunlu olmamalı.
- **Güvenlik varsayılan:** Eşleşmemiş cihazlar panoya erişememeli; veri bağlantısı şifreli olmalı.
- **Açık kaynak / şeffaf:** Kaynak kodu, protokol ve katkı dokümanları GitHub'da yayımlanmalı.

## 3. Destek matrisi ve gerçek platform sınırları

| Platform | İlk hedef | Uzun vadeli hedef | Kritik not |
|---|---|---|---|
| Windows | Otomatik metin, ardından görsel | Dosya, geçmiş, çoklu cihaz | Pano olayları ve uygulama başlangıcı test edilmeli |
| macOS | Otomatik metin, ardından görsel | Dosya, geçmiş, çoklu cihaz | İzinler, kod imzalama ve notarization planlanmalı |
| Linux | V0.2 | Tam masaüstü desteği | Wayland ve X11 pano davranışları farklıdır |
| Android | V1.0 | Paylaşım menüsü, hızlı gönder/al | Android 10+ arka planda pano okuma kısıtları var; sistem ayrıcalığı olmadan evrensel otomatik pano vaat edilmemeli |
| iOS / iPadOS | V1.0 | Share Extension, uygulama içi pano akışı | Arka planda panoyu sessizce sürekli okuma kısıtlıdır; Apple'ın yerleşik süreklilik özelliği birebir kopyalanamaz |

**Önemli:** Masaüstü için “otomatik çift yönlü pano” temel ürün vaadi olabilir. Mobil için aynı vaadi teknik doğrulama olmadan vermeyiz; hızlı, kullanıcı kontrollü paylaşım akışları tasarlarız.

## 4. İlk sürüm (MVP) — kesin kapsam

### V0.1 zorunlu özellikler

1. Windows ve macOS masaüstü istemcileri.
2. Aynı Wi-Fi / yerel ağda mDNS/Bonjour ile cihaz keşfi; keşif başarısızsa elle IP veya QR tabanlı alternatif.
3. Kullanıcının açık onayıyla tek seferlik güvenli cihaz eşleştirme.
4. Düz metin (UTF-8) panosunun **iki yönde**, otomatik aktarımı; tek eş cihaz.
5. Eşzamanlı kopyalama durumunda öngörülebilir çakışma yönetimi.
6. Döngü önleme: Uzaktan gelen pano değişikliği yeniden karşıya gönderilmemeli.
7. Bağlantı kesilince yeniden keşif/bağlanma; bilgisayar uykudan uyanınca toparlanma.
8. Menü çubuğu (macOS) ve sistem tepsisi (Windows); uygulama girişte otomatik başlayabilmeli.
9. Eşleşmiş cihazları görme, geçici durdurma, eşleşme kaldırma, son aktarım durumu.
10. Hassas veriler için temel gizlilik kontrolleri: pano izlemeyi durdurma, kayıtları sınırlama ve gizli alanları loglamama. Parola yöneticilerinin "gizli/geçici" işaretli kopyaları hiç gönderilmez.

### V0.1.1

- PNG başta olmak üzere görsel panosu, sonra yaygın görsel biçimleri.
- Büyük görsellerde bellek sınırı ve akış bazlı aktarım.
- Uygulama bazlı hariç tutma seçenekleri / isteğe bağlı hassas pano algılama (yanlış pozitif/negatif testleriyle).

### İlk sürümde *yok*

- Bulut kullanıcı hesabı, merkezi relay, otomatik internet üzerinden bağlantı.
- Fare/klavye paylaşımı.
- Dosya aktarımı ve pano geçmişi.
- Zengin metin (HTML/RTF) ve çoklu cihaz senkronizasyonu.
- Çevrimdışı cihaz için içerik kuyruğu.
- Mobilde arka planda sınırsız otomatik pano iddiası.

## 5. Kullanıcı yolculuğu

1. İki cihazda uygulama açılır.
2. “Yakındaki cihazlar” alanında birbirlerini görürler.
3. Kullanıcı birine dokunur; her iki uçta eşleştirme onayı ve doğrulama kodu/QR gösterilir.
4. Doğrulama başarılı olunca anahtarlar güvenli biçimde saklanır.
5. Uygulama arka planda açık kalır; kullanıcı başka pencere açmadan normal `Cmd/Ctrl+C` ve `Cmd/Ctrl+V` ile çalışır.
6. Sonraki açılışlarda onay tekrar istenmez; kullanıcı istediğinde eşleşmeyi iptal edebilir.

İdeal ekranlar yalnızca **Cihazlar**, **Durum**, **Ayarlar**, **Hakkında**. Zorunlu sunucu kurulum ekranı yok.

## 6. Önerilen teknik mimari

### Teknoloji seçimi (Rust + Tauri 2 kesinleşti; platform detayları prototipte doğrulanacak)

- **Ortak çekirdek:** Rust (protokol, cihaz keşfi, eşleştirme, durum makinesi, taşıma, veri modeli).
- **Masaüstü kabuğu:** Tauri 2 (Windows/macOS; Linux sonradan).
- **Platform adaptörleri:** Windows ve macOS yerel pano API'leri. Olay tabanlı izleme mümkünse tercih edilmeli, gerekiyorsa düşük maliyetli kontrollü polling.
- **Keşif:** mDNS / DNS-SD; farklı VLAN'larda engellenebileceğinden elle adres seçeneği.
- **Taşıma:** İlk MVP için doğrudan LAN TCP üstünde kimliği doğrulanmış, şifreli kanal (ör. rustls tabanlı TLS); daha gelişmiş bağlantı yöntemleri sonraya.
- **Cihaz kimliği:** Her cihazın yerel ürettiği kalıcı anahtar çifti; ilk eşleştirmede kısa doğrulama kodu/QR ile karşı tarafın anahtarını doğrulama.
- **Gizli anahtar saklama:** OS credential storage / Keychain / Windows Credential Manager.
- **Veri formatı:** Sürüm numaralı, açık bir ikili protokol; metin için UTF-8, görsel için MIME türü + boyut + içerik.

> **“Sunucusuz” tanımı:** Merkezi, ayrı bir sunucu yok. Eşleşmiş uygulamaların biri yerel ağ portu dinleyebilir; bu P2P mimaride normaldir. Kullanıcı bunu kurmak veya yönetmek zorunda kalmaz.

### Mantıksal bileşenler

```text
┌──────────────────────┐                ┌──────────────────────┐
│ macOS uygulaması     │                │ Windows uygulaması  │
│ - Yerel pano adaptörü│                │ - Yerel pano adaptörü│
│ - Menü çubuğu UI     │                │ - Sistem tepsisi UI  │
│ - Rust çekirdeği     │◄═══ TLS/P2P ═►│ - Rust çekirdeği     │
│ - Cihaz anahtarı     │                │ - Cihaz anahtarı     │
└──────────────────────┘                └──────────────────────┘
           ▲                                       ▲
           └──────── mDNS cihaz keşfi ─────────────┘
```

### Temel olay akışı

1. OS panosunda değişiklik algılanır.
2. İçerik türü ve boyutu doğrulanır; politika filtresinden geçirilir.
3. İçerik özeti (`hash`) ve `event_id` hesaplanır.
4. Yeni yerel olay eşleştirilmiş/çevrimiçi cihazlara şifreli gönderilir.
5. Alan cihaz `event_id` ile tekrarları ve döngüleri eler; gerekirse panosuna uygular.
6. Uygulanan uzak olayın tekrar yerel olay gibi yayınlanması engellenir.
7. Karşı taraf çevrimdışıysa içerik kuyruğa alınmaz, unutulur; cihaz yeniden bağlandığında eski içerik panonun üzerine yazılmaz.

### Çakışma stratejisi

İki cihaz neredeyse aynı anda kopyalarsa “en yeni olay kazanır” yaklaşımı tek başına güvenilir değildir (saat kaymaları). MVP'de **her cihazda son yerel kopya öncelikli** bir durum makinesi tasarlanmalı; uzak olayların sırası mantıksal sayaçlar ve cihaz kimlikleriyle belirlenmeli. Çakışma örnekleri otomasyon testlerine girmeli. Kullanıcının az önce kopyaladığının gecikmiş uzak veriyle beklenmedik biçimde ezilmemesi temel ölçüttür.

### Güvenlik kararları

- Eşleştirme sırasında MITM'e karşı kısa kimlik doğrulama kodu / QR üzerinden anahtar doğrulaması.
- Güvenilmeyen cihazdan alınan tüm içerik doğrulanır; boyut limitleri vardır.
- Varsayılan olarak internete açık port, bulut hesabı veya gizli telemetri bulunmaz.
- Pano içeriği **loglanmaz**, tanılama kayıtları anonim/minimal tutulur.
- Eşleşme iptal edilince ilgili anahtar/erişim kaldırılır.
- Parolalar, OTP'ler, hasta verileri gibi hassas içeriklerin riskleri kullanım arayüzünde açıklanır; güvenli duraklatma kolay erişilebilir olur.
- Kriptografi sıfırdan yazılmaz; denetlenmiş kütüphane kullanılır.

## 7. Açık kaynak stratejisi

- Kod, protokol ve katkı yönergeleri herkese açık GitHub reposunda.
- Lisans: **MIT**.
- `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `PRIVACY.md` dosyaları.
- Bağımlılık lisans taraması, güvenlik güncelleme politikası.
- Windows ve macOS için otomatik derleme; sürüm yayınında imza ve macOS notarization süreci planla.
- Ad/marka uygunluğunu ve mevcut proje isimlerini doğrulamadan resmî markalama yapma.

## 8. Önerilen monorepo dizinleri

```text
opencontinuity/
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
├── docs/
│   ├── PRD.md
│   ├── architecture.md
│   ├── protocol.md
│   ├── security-model.md
│   └── compatibility.md
├── crates/
│   ├── core/                # durum makinesi, senkronizasyon, deduplikasyon
│   ├── discovery/           # mDNS / manuel keşif
│   ├── transport/           # şifreli eşler arası iletişim
│   ├── pairing/             # kimlik, güven, anahtar yönetimi
│   ├── clipboard/           # ortak arayüz + platform adaptörleri
│   └── protocol/            # sürümlü mesaj tipleri
├── apps/
│   └── desktop/             # Tauri 2 UI + yerel entegrasyonlar
├── tests/
│   ├── integration/
│   ├── interoperability/
│   └── resilience/
└── .github/workflows/       # lint, test, build, sürümler
```

## 9. Geliştirme yol haritası

### Faz 0 — Araştırma / PoC

- [ ] Apple/macOS, Windows pano API'leri ve izin davranışlarını örnek uygulamalarla doğrula.
- [ ] İki cihaz arasında mDNS keşfi prototipi.
- [ ] Eşleştirme + doğrulanmış TLS bağlantısı prototipi.
- [ ] Metin panosunu **fare taşıma olmadan** iki yönlü aktar.
- [ ] Ağ yeniden bağlanma ve pano döngüsü testleri.

### Faz 1 — V0.1 masaüstü MVP

- [ ] Tauri masaüstü kabukları.
- [ ] İlk eşleştirme arayüzü.
- [ ] Güvenilir çift yönlü metin aktarımı.
- [ ] Otomatik başlatma / gizli arka plan çalışması.
- [ ] Kopma/uyku/yeniden başlatma senaryoları.
- [ ] Windows installer + macOS dağıtımı (ilk test sürümleri).

### Faz 2 — V0.1.1 ve V0.2

- [ ] Görsel panosu ve MIME uyumluluğu.
- [ ] Çoklu cihaz / grup senkronizasyonu.
- [ ] Linux X11 + Wayland uyumluluk araştırması ve destek.
- [ ] Dosya paylaşımı / sürükle-bırak (ayrı, açık kullanıcı eylemi).
- [ ] Gelişmiş kullanıcı tercihleri ve pano geçmişi (varsayılan kapalı değerlendir).

### Faz 3 — V1.0 mobil ve geniş ekosistem

- [ ] Android paylaşım menüsü, hızlı erişim, izin sınırları içinde pano akışı.
- [ ] iOS/iPadOS Share Extension ve kullanıcı kontrollü pano al/gönder.
- [ ] Mobil ↔ masaüstü dosya/görsel gönderimi.
- [ ] Farklı ağlar için **isteğe bağlı** relay (ör. yereldeki Linux sunucusu) ve açık sunucu protokolü.

## 10. Kalite / başarı kriterleri

“En iyi” diyebilmek için sadece çalışıyor olması yetmez. Ölçülebilir kabul testleri:

- Temiz kurulum: Kullanıcı IP/port girmeden aynı ağdaki iki cihazı bulup eşleştirebiliyor.
- Mac → Windows ve Windows → Mac metin kopyala/yapıştır, fare konumundan bağımsız çalışıyor.
- 1.000 tekrarlı pano olayında sonsuz döngü, kilitlenme, kaynak sızıntısı yok.
- 24 saat arka plan çalışmasında kararlı bellek/CPU kullanımı (hedefler PoC ölçümüyle belirlenecek).
- Ağ kopması, Wi-Fi değişimi, ekran kilidi, uyku/uyanma, VPN, güvenlik duvarı, yeniden başlatma testleri.
- 0 bayt, Unicode/Türkçe, çok uzun metin, emoji, satır sonları, zengin metin ve görsel panosu testleri.
- Güvenilmeyen/eşleşmemiş cihazın içerik okuyamaması.
- Hata ekranları anlaşılır; geliştirici konsolu açmak gerekmiyor.
- Gecikme, CPU/RAM ve başarı oranları ölçülmeden “anında” veya “kusursuz” iddiası kullanılmaz.

## 11. Vibe coding ile çalışma kuralları

1. Önce uygulama iskeleti değil, **protokol ve güvenlik varsayımları** yazılsın.
2. Her özellik için küçük bir issue / görev / kabul kriteri olsun.
3. Ajan yeni kod yazmadan önce mevcut mimariyle çakışmaları tespit etsin.
4. Her PR'da unit/integration testleri; Windows ve macOS üzerinde gerçek cihaz testi gerektiren maddeler ayrı işaretlensin.
5. İzin ve OS API sınırlarını “aşılmış” gibi varsaymasın; mümkün değilse açıkça raporlasın.
6. Gizli pano verisi örnek loglara/test kayıtlarına girmesin.
7. Önemli güvenlik kararları insan incelemesinden geçsin.
8. Önce metin panosu gerçekten çalışsın; ardından görsel, sonra diğer özellikler.

### Geliştirme ajanına verilecek ilk görev (kopyala-yapıştır)

> Bu depodaki `OpenContinuity_Proje_Plani.md` belgesini ürün kapsamı için temel kabul et. Önce Windows ve macOS arasında, aynı yerel ağ üzerinde, merkezi sunucu gerektirmeden, kullanıcı kontrollü ilk eşleştirme ve şifreli doğrudan bağlantıyla çift yönlü **metin pano senkronizasyonu** yapan bir PoC tasarla. Fare/klavye paylaşımı ekleme. İlk yanıtta kod yazmadan önce: mimari seçenekler, platform API'leri, güvenlik tehdit modeli, bilinmeyenler, test planı, fazlara ayrılmış görev listesi ve teknik riskleri açıkla. mDNS başarısızsa manuel eşleştirme yedeğini tasarla. Sistem panosu olayları, karşılıklı güncelleme döngüsü, çakışma, uyku/uyanma, bağlantı kopması, Unicode/Türkçe içeriği özel olarak ele al. Ardından en küçük test edilebilir kod değişiklikleriyle ilerle; gerçek cihaz testleri yapılmadıysa yapılmış gibi sunma.

## 12. Kararlar

### Verilen kararlar (17 Ekim 2026)

- [x] **Hedef kitle:** Önce Behcet'in kendi kullanımı (kişisel + klinik); repo baştan herkese açık.
- [x] **Çalışma adı:** OpenContinuity ile başlanır; ad/marka kontrolü yayın öncesi yapılır.
- [x] **Lisans:** MIT.
- [x] **Eşleştirme UX'i:** Altı haneli doğrulama kodu ana yol, QR ek seçenek.
- [x] **Cihaz sayısı:** V0.1'de tek eş cihaz; çoklu cihaz Faz 2'de.
- [x] **İçerik türü:** V0.1'de yalnız düz metin (UTF-8); HTML/RTF zengin metin sonraya.
- [x] **Hassas içerik:** Parola yöneticilerinin "gizli/geçici" işaretli kopyaları (macOS `org.nspasteboard.ConcealedType` / `TransientType`, Windows `ExcludeClipboardContentFromMonitorProcessing` / `CanIncludeInClipboardHistory=0`) hiç gönderilmez; diğer içerik gönderilir.
- [x] **Çevrimdışı davranış:** Karşı cihaz kapalıyken kopyalanan içerik kuyruğa alınmaz, unutulur; yeniden bağlanınca eski içerik panoyu ezmez.
- [x] **Çakışma politikası:** Her cihazda son yerel kopya önceliklidir (bkz. Bölüm 6 "Çakışma stratejisi").
- [x] **Teknoloji yığını:** Rust çekirdek + Tauri 2 kesinleşti.

### Açık sorular

- [ ] Mevcut alternatiflerden (KDE Connect, LocalSend, ClipCascade vb.) farkımızın tek cümlelik tanımı (README için).
- [ ] Test cihazları: Windows ve macOS sürümleri; aynı Wi-Fi mı, klinik ağında VLAN/misafir ağı ayrımı var mı?
- [ ] Apple Developer hesabı (notarization için, yıllık 99 $) ve Windows kod imzalama sertifikası alınacak mı; ilk sürümlerde SmartScreen uyarısı kabul mü?
- [ ] GitHub konumu: kişisel hesap mı, yeni organizasyon mu?
- [ ] CI ve sürümleme düzeni.

---

### Projenin altın kuralı

**Uygulamanın varlığını unutabildiğin kadar iyi çalışmalı.** Kullanıcının bir kez bile IP, port, sertifika veya sunucu ayarı görmek zorunda kaldığı akış, varsayılan deneyim sayılmaz.

*Not: Bu dosya bir ürün ve teknik başlangıç planıdır; uygulanmış/çalıştığı doğrulanmış bir yazılım değildir.*
