# OpenContinuity için iOS: Arka Plan Panosu, App Store Sınırları ve Önerilen Mimari

## Sonuç ve kritik bulgular

19 Ekim 2026 itibarıyla araştırmanın en önemli sonucu şu:

**iOS 26/27’de, normal bir üçüncü taraf App Store uygulamasının iPhone/iPad genel panosunu (`UIPasteboard.general`) uygulama askıdayken sürekli izlemesini ve her `Copy` işleminde otomatik olarak LAN’daki bilgisayarlara göndermesini sağlayan genel kullanıma açık bir API yok.** Apple’ın kendi Universal Clipboard özelliği bunu yapabiliyor çünkü bir App Store uygulaması gibi çalışmıyor; Handoff/Continuity’nin işletim sistemi seviyesindeki servislerinden yararlanıyor. Apple’ın güvenlik belgeleri Universal Clipboard’ın Handoff altyapısını kullandığını doğruluyor. Üçüncü taraf uygulamalar ise iOS uygulama yaşam döngüsü, sandbox ve izin verilen background-mode kurallarıyla sınırlı. citeturn4search9turn17search3turn23search10

Bu fark yalnızca teorik değil. Paste kendi iOS yardım sayfasında açıkça, iOS uygulamalarının arka plandayken clipboard’a erişemediği için kopyalanan içeriği o anda otomatik yakalayamadığını söylüyor. Microsoft da 2026’da Apple’a yaptığı DMA interoperability başvurusunda aynı engeli gerekçe gösterdi: mevcut iOS kısıtlamalarının iPhone ile PC arasında arka planda kesintisiz clipboard sync yapılmasını engellediğini belirtti. citeturn19search29turn19news34

Bununla birlikte **App Store’a tamamen uygun, pratikte “bir hareketlik” bir OpenContinuity deneyimi yapmak mümkün**:

> **iPhone → bilgisayar:** Action Button / Back Tap / Shortcuts widget / Control Center üzerinden  
> `Get Clipboard → OpenContinuity Send Intent`  
>   
> **bilgisayar → iPhone:** aynı yüzeylerden  
> `OpenContinuity Fetch Latest → Copy to Clipboard`

Apple, `Get Clipboard` ve `Copy to Clipboard` işlemlerinin Shortcuts widget’ından Shortcuts uygulamasını öne getirmeden çalışabilecek işlemler arasında olduğunu özellikle belgeliyor. Dolayısıyla OpenContinuity’nin kendi background App Intent’i yalnızca ağ transferini yapar; clipboard okuma/yazma işini kullanıcı niyeti açık olan Apple Shortcuts işlemleri yapar. Bu, mevcut iOS’ta bulabildiğim en temiz ve App Review açısından en güvenli “near-automatic” çözüm. citeturn21search1turn21search4turn6search2turn6search18

Ancak bunun **“her Copy olayında otomatik tetiklenen”** bir sürümü yok. Shortcuts’ın güncel otomasyon tetikleyicileri arasında genel bir **Clipboard Changed** olayı bulunmuyor. iOS 27’de otomasyon yüzeyleri genişlemiş olsa da clipboard değişikliğini dinleyen genel bir trigger eklenmiş değil. citeturn7search7

Daha da önemlisi, araştırma sırasında OpenContinuity açısından çok değerli yeni bir 2026 gelişmesi ortaya çıkıyor: **Apple, Microsoft’un AB DMA kapsamında yaptığı cross-device clipboard interoperability talebini kabul etti.** Microsoft’un talebi 25 Mart 2026’da yapılmış; basına yansıyan Apple yanıtı 26 Haziran 2026 tarihli. Apple, gelecekte sistemin clipboard değişikliklerinde bir extension’ı haberdar edeceği ve extension’ın eşleştirilmiş aksesuarla pasteboard öğelerini paylaşabileceği/ithal edebileceği yeni bir çözüm geliştireceğini söylüyor. Apple bunun “önemli bir mühendislik çalışması” olduğunu, geliştirmeyi **2027 sonbaharında tamamlayıp ardından developer beta** vermeyi hedeflediğini belirtiyor. Yani bu yetenek **bugünkü iOS 27’de henüz yok**, ama OpenContinuity’nin aradığı şeyin neredeyse birebir resmi yol haritası. citeturn18search13turn18news27turn19news34

Bu nedenle nihai teknik tavsiyem:

**iOS uygulamasını native SwiftUI + mevcut Rust engine crate’in UniFFI üzerinden kullanıldığı hibrit bir yapı olarak geliştirin. Tauri mobile’ı iOS ana mimarisi yapmayın.** Tauri 2 iOS’u destekliyor ve Swift plugin’leri yazılabiliyor; fakat sizin iOS uygulamanızın zor tarafı web UI değil, Share Extension, App Intents, Widget/Control yüzeyleri, Photos/Files, Local Network, pasteboard ve gelecekteki DMA accessory extension entegrasyonları. UniFFI ise Rust kütüphaneleri için doğrudan Swift binding üretmek üzere tasarlanmış durumda. citeturn20search0turn20search4turn20search8turn20search1turn20search5

## Apple’ın Universal Clipboard ayrıcalığı ve iOS pano modeli

### Apple neden yapabiliyor, üçüncü taraf neden yapamıyor?

Universal Clipboard, “Apple’ın `UIPasteboard` kullanan gizli bir uygulaması” değildir. Apple bunu Handoff/Continuity sistem altyapısına entegre etmiş durumda. Universal Clipboard için aynı Apple Account, Wi‑Fi, Bluetooth ve Handoff gerekiyor; Apple’ın güvenlik dokümantasyonu da Universal Clipboard’ın Handoff mekanizmasını kullandığını açıkça belirtiyor. citeturn4search9turn17search1turn17search3

Buradaki kritik mimari fark şudur:

```text
Üçüncü taraf uygulama

OpenContinuity.app
      │
      ├── foreground → çalışır
      ├── kısa background süresi → bazen çalışır
      └── suspended → CPU yok, polling yok, socket listener yok
                            │
                            X clipboard değişimini göremez


Apple Continuity

Copy olayı
   │
   ▼
iOS sistem pasteboard / Continuity altyapısı
   │
   ├── Handoff
   ├── Wi‑Fi / Bluetooth
   ├── sistem daemon'ları
   └── diğer Apple cihazı
```

Üçüncü taraf uygulama askıya alındığında artık kendi timer’ı, `changeCount` poll’u, Bonjour browser’ı veya socket listener’ı çalıştırılamaz. Apple Developer Forums’taki Networking açıklamalarında da uygulama background’a geçince askıya alınabileceği ve askıya alındığında listener socket üzerinden gelen bağlantıları işleyemeyeceği açıkça belirtiliyor. citeturn23search10

Avrupa Komisyonu’nun Apple’a ilişkin DMA interoperability incelemesi de aynı yapısal asimetriyi ele aldı: Apple’ın kendi bağlı-cihaz işlevlerinde sistem seviyesinde sürekli çalışan bileşenler kullanabildiği, üçüncü taraf companion app’lerin ise normal background yaşam döngüsüyle sınırlı kaldığı tespit edildi. Bu, neden yalnızca bir `UIBackgroundModes` anahtarı ekleyerek Universal Clipboard eşdeğeri yapılamadığını açıklıyor. citeturn14view0

Dolayısıyla sorun aslında iki ayrı kilitten oluşuyor:

**Birinci kilit: pano.** Üçüncü tarafın background clipboard-change subscription’ı yok.

**İkinci kilit: çalışma zamanı.** Clipboard’a erişebilecek kodunuz olsa bile iOS sizi her zaman çalıştırmıyor.

OpenContinuity için gerçek otomatik sync ancak bu iki kilidin ikisinin de açılmasıyla mümkün olur.

### `UIPasteboard.general` gerçekte ne kadar “foreground only”?

Burada önemli bir terminoloji ayrımı yapmak gerekiyor. `UIPasteboard` API’si üzerinde “bu fonksiyon yalnızca foreground’da derlenebilir” şeklinde basit bir annotation yoktur. API hem okuma hem yazma metodları sunar. Problem şudur: **iOS normal uygulamanıza clipboard değiştiğinde background çalışma hakkı vermez; uygulama suspended olunca hiçbir kodunuz çalışmadığı için clipboard’ı gözlemleyemezsiniz.** Ayrıca extension/background process’lerinde general pasteboard erişiminin başarısız olduğuna dair Apple forumlarında `PBErrorDomain` örnekleri de bulunuyor. citeturn17search11turn3search0

Bu yüzden pratik kural:

**Uygulama aktifken:** evet, okuyun/yazın.

**Foreground’dan yeni ayrılmış ve geçici background execution süresindeyken:** bazı işlemler teknik olarak gerçekleşebilir; fakat buna dayalı bir ürün tasarlamayın.

**Uygulama suspended iken:** polling veya clipboard monitor yok.

**Clipboard değişince uygulamayı uyandır:** public API yok.

Bu, Paste’in kendi dokümantasyonunda “apps can’t access the clipboard while running in the background” şeklinde özetlediği sınırlamayla örtüşüyor. citeturn19search29

### Paste izni neden ikinci bir engel?

iOS 14 ile Apple, başka uygulamadan gelen general pasteboard içeriğinin programatik okunmasını kullanıcıya görünür hale getirdi. iOS 16’da bu politika daha ileri taşındı ve kullanıcı niyetiyle ilişkilendirilmeyen programatik pasteboard okumaları için izin alert’i getirildi. Buna karşılık kullanıcının doğrudan etkileştiği sistem paste yüzeyleri “user intent” kabul ediliyor. Apple bu amaçla `UIPasteControl` ekledi. citeturn1search1turn9search1turn23search21

Bu yüzden uygulamayı açıp arka planda her 300 ms’de bir:

```swift
UIPasteboard.general.string
```

okumak yalnızca pil açısından kötü değildir; iOS’un clipboard privacy modelinin tam tersidir.

`UIPasteControl`, kullanıcı düğmeye dokunduğunda pasteboard içeriğini hedef nesneye aktarır ve bu açık kullanıcı niyeti nedeniyle normal paste alert’ini tetiklemeden kullanılabilir. citeturn23search1turn23search21

OpenContinuity foreground ekranında bu nedenle şu seçenek güzel olur:

**“Panoyu gönder”** için sistem `UIPasteControl` tabanlı yüzey veya açık kullanıcılı paste akışı.

Bunun yanında kullanıcı uygulamaya daha kalıcı paste izni verdiyse foreground auto-sync modu sunabilirsiniz; fakat uygulama suspended olduğunda yine sorun çözülmez.

### `changeCount` bir event stream değildir

`UIPasteboard.changeCount`, pasteboard’ın kaç kez değiştiğini belirten sayaçtır. Bir background wake-up API’si değildir. Uygulama zaten çalışıyorsa son gördüğünüz sayıyla karşılaştırabilirsiniz; suspended uygulamayı uyandırmaz. Apple ayrıca değişiklik sayısının event-loop davranışı nedeniyle birden fazla değişikliği gruplayabileceğini belirtiyor. citeturn1search3turn5search5

Dolayısıyla foreground watcher için iyi bir optimizasyon:

```text
poll changeCount
   │
   ├── aynıysa → payload okuma
   │             yapma
   │
   └── değiştiyse → gerekli pasteboard tipini oku
                    hash/dedup
                    gönder
```

Ama bu hâlâ **foreground watcher**.

### `detectPatterns` ve `detectValues` izin sorununu çözmez

Apple’ın pasteboard detection API’leri, tam payload’ı uygulamaya vermeden “URL olabilir”, “telefon numarası olabilir” vb. tip/pattern tespiti yapabilmek için tasarlandı. Eski `detectPatterns` belgeleri özellikle bunun kullanıcıya pasteboard erişim bildirimi oluşturmadan yapılabildiğini belirtiyor. Yeni `detectValues`/`detectedValues` ailesi de aynı privacy-preserving yaklaşımın devamı. citeturn1search0turn1search5turn1search11

Fakat:

> “Clipboard’da URL var” bilgisi sync için yeterli değildir.  
> OpenContinuity’nin asıl string/image/file bytes’ına ihtiyacı vardır.

Dolayısıyla bunlar ön kontrol optimizasyonu olabilir, Universal Clipboard workaround’u değil.

## iOS yüzeylerinin teknik ve App Store matrisi

Aşağıdaki tabloda **“arka plan”**, “uygulama UI’si görünmüyor ama sistem bu işi desteklenen bir mekanizmayla çalıştırabiliyor mu?” anlamında kullanılmıştır. “Risk” ise App Review sonucu için garanti değil; API’nin amaçlanan kullanımına ne kadar yakın olduğuna ilişkin değerlendirmedir.

| Mekanizma | Clipboard READ | Clipboard WRITE | Ana uygulama foreground gerekli mi? | OpenContinuity açısından sonuç / App Review riski |
|---|---|---|---|---|
| `UIPasteboard.general` | ✅ İçerik okunabilir; başka uygulamadan gelen içerikte paste privacy politikası geçerli | ✅ | Güvenilir kullanım için uygulama aktif olmalı; suspended app clipboard watcher olamaz | **Foreground auto-sync için uygun.** Background auto-sync için uygun değil. Risk: düşük/orta. citeturn17search11turn19search29 |
| `changeCount` | ⚠️ Yalnızca değişiklik sayısı; payload değil | — | Evet, kodunuzun çalışıyor olması gerekir | Foreground polling optimizasyonu; wake-up mekanizması değildir. Risk düşük. citeturn1search3turn5search5 |
| `detectPatterns` / `detectValues` | ⚠️ Pattern/semantic detection; arbitrary payload değil | ❌ | Kodun çalışıyor olması gerekir | Paste alert’inden kaçınarak içeriğin türünü önceden anlamaya yarar; sync yapamaz. Risk düşük. citeturn1search0turn1search5 |
| `UIPasteControl` | ✅ Kullanıcı dokunuşuyla | ❌ Temel amacı paste almak | Evet; UI kontrolüdür | En App-Store-dostu clipboard okuma yüzeylerinden biri. Alert yerine explicit user intent. citeturn23search1turn23search21 |
| iOS 26 “New from Clipboard” standard command | ✅ Explicit command ile | ❌ | Evet | iOS 26’da sistem “New from Clipboard” komutu alert oluşturmadan belge yaratmak için geldi; background watcher değildir. citeturn21search2 |
| `BGTaskScheduler` | ⚠️ Clipboard watcher olarak desteklenmiyor | ⚠️ Aynı nedenle güvenilmez | Hayır, fakat zamanlamayı sistem belirler | Periyodik bakım/sync içindir; “her Copy’den sonra çalış” garantisi vermez. Clipboard monitoring için kullanmayın. citeturn0search0turn0search11 |
| `BGContinuedProcessingTask` — iOS 26+ | ❌ Yeni clipboard yetkisi vermez | ❌ Yeni clipboard yetkisi vermez | Kullanıcı foreground’da işi başlatabilir, sonra devam edebilir | **500 MiB kullanıcı başlatımlı transferler için değerli.** Clipboard daemon yapmak için değil. Risk düşük, amacına uygun kullanılırsa. citeturn0search9turn0search5 |
| Background audio | Clipboard problemi teorik olarak process canlı tutulursa azalır, fakat… | …aynı | Hayır | **Clipboard için silent-audio hack kullanmayın.** Guideline 2.5.4 ve güncel review uygulaması açısından yüksek risk. citeturn10search1turn10search14 |
| Background location | Aynı şekilde process runtime sağlayabilir ama clipboard kullanım amacı değildir | Aynı | Hayır | Clipboard utility’nin konumu sürekli istemesi 2.5.4 ve privacy açısından savunulamaz. **Yüksek red riski.** citeturn10search1 |
| Local Network + Bonjour | Clipboard API’si değil | Clipboard API’si değil | Discovery/listener foreground’da güvenilir; suspended olunca generic listener işlemez | LAN transport için gerekli. `NSLocalNetworkUsageDescription` + `NSBonjourServices`; izin foreground’da alınmalı. Risk düşük. citeturn0search4turn0search1turn23search10 |
| Wi‑Fi Aware — yeni nesil API | ❌ Clipboard yetkisi yok | ❌ | Paired Wi‑Fi Aware cihazlarıyla bazı bağlantılar foreground/background çalışabilir | Gelecekte transport açısından araştırmaya değer; **pasteboard engelini kaldırmaz.** citeturn15search4turn17search14 |
| Custom Keyboard | ⚠️ Extension aktifken clipboard ile etkileşim mümkün olabilir; özel background entitlement değildir | ⚠️ Aynı | Ana app değil ama **keyboard ekranda/aktif olmalı** | `RequestsOpenAccess` network/shared container sağlar; sürekli watcher sağlamaz. Secure fields/host restrictions da vardır. MVP’ye koymayın. citeturn15search0turn9search5 |
| Share Extension | Kullanıcının paylaştığı `NSExtensionItem` içeriğini ✅ alır; arbitrary clipboard monitor değildir | General clipboard’a yazmayı temel mimari yapmayın | Ana app foreground olmak zorunda değil; extension kullanıcı tarafından açılmıştır | **iPhone→PC için en önemli mekanizmalardan biri.** Düşük risk. citeturn6search0turn6search8turn20search18 |
| Action Extension | Host’un sağladığı içeriği ✅ işleyebilir | Host’a dönüştürülmüş içerik döndürebilir; clipboard daemon değildir | Kullanıcı extension’ı çağırır | Share Extension kadar doğal değil; clipboard sync için özel avantajı yok. Düşük risk. citeturn6search4 |
| App Intent tek başına | ❌ Özel background clipboard entitlement yok | ❌ Özel entitlement yok | Intent foreground veya background modunda çalışabilir | Ağdaki OpenContinuity peer’ına **bir kerelik** send/fetch için çok uygun. Clipboard’ı Shortcuts’a bırakın. citeturn6search2turn6search3turn20search15 |
| Shortcuts `Get Clipboard` | ✅ | — | **Ana OpenContinuity app foreground olmak zorunda değil** | En değerli workaround. Widget’tan bile content alıp işleyebilir/gönderebilir. citeturn21search1turn21search4 |
| Shortcuts `Copy to Clipboard` | — | ✅ | Ana OpenContinuity app foreground olmak zorunda değil | Mac→iPhone için mükemmel son adım. `Local Only` ve expiration seçenekleri de var. citeturn21search1 |
| Shortcuts automation | ✅ Trigger çalıştıktan sonra `Get Clipboard` kullanılabilir | ✅ | Hayır | Fakat **genel Clipboard Changed trigger yok**; her Copy otomatik tetiklenemez. citeturn7search7turn21search1 |
| Interactive Widget — iOS 17+ | ❌ Doğrudan özel clipboard ayrıcalığı yok | ❌ | Widget App Intent çağırabilir | OpenContinuity widget’ından çok **Shortcuts widget** clipboard send/fetch için daha güçlü. citeturn8search0turn8search12 |
| Control Center Control — iOS 18+ | ❌ Doğrudan yok | ❌ | App Intent çalıştırabilir | Bir “Send/Fetch” intent’i tetiklemek için iyi; clipboard gerekiyorsa Shortcuts control daha uygun. citeturn8search1turn8search5 |
| Action Button | ❌ Kendi başına yok | ❌ | Hayır | Shortcut çalıştırabildiği için **bir basışta sync** için mükemmel. citeturn8search2turn8search6 |
| Back Tap | ❌ Kendi başına yok | ❌ | Hayır | Shortcut’a bağlanabilir; ekstra donanım gerektirmeyen bir-gesture seçenek. citeturn8search3turn8search7 |
| Live Activities | ❌ | ❌ | Hayır | Clipboard listener değil. Aktif büyük dosya transferinin durumunu göstermek için ileride faydalı; interactive action yine App Intent üzerinden. citeturn8search0 |
| Notification Service Extension | **Desteklenen bir general-pasteboard background yolu değil** | **Desteklenen/güvenilir bir background clipboard-write yolu değil** | App foreground değildir | Push’ı clipboard-daemon wake hack’i olarak kullanmayın. Extension context’lerinde pasteboard erişiminin çalışmadığı örnekler mevcut. citeturn15search1turn3search0 |
| Notification Content Extension | ❌ Clipboard ayrıcalığı yok | ❌ | Hayır | Notification UI içindir; clipboard sync çözümü değildir. |
| Silent push | Clipboard yetkisi sağlamaz | Clipboard yetkisi sağlamaz | Hayır | Teslim garantili değildir, APNs/provider gerektirir ve “no cloud” mimarinize de ters. Clipboard çözümü değil. |
| Continuity Camera | ❌ | ❌ | — | Continuity ailesinin üyesi olması, üçüncü taraf uygulamaya Handoff/Universal Clipboard daemon erişimi vermez. citeturn4search12 |
| Gelecekteki AB cross-device clipboard extension | **Planlanan çözümde evet: sistem clipboard değişiminde extension haberdar olacak** | **Import yolu da planlanıyor** | Sistem extension’ı uyandıracak şekilde tasarlanıyor | OpenContinuity için gerçek çözüm olabilir; **iOS 27’de henüz mevcut değil**, 2027 sonbaharı sonrası developer beta hedefleniyor. citeturn19news34turn18news27 |

### Background audio/location hilesi özellikle kaçınılması gereken alan

Apple’ın App Review Guideline 2.5.4 yaklaşımı, background mode’ların yalnızca tanımlandıkları gerçek kullanım için kullanılmasını istiyor: audio gerçekten audio içindir, location gerçekten konum gerektiren uygulamalar içindir. Apple Developer Forums’ta Temmuz 2026 tarihli bir review örneğinde, `audio` background mode beyan edilmiş olmasına rağmen reviewer uygulamada persistent audio gerektiren bir fonksiyon bulamadığı için uygulama 2.5.4 altında reddediliyor. citeturn10search1turn10search14

Bu konuda tarihsel olarak çok öğretici bir örnek de **Pastebot**. Eski iPhone Pastebot sürümü, background’da clipboard izlemek için sessiz bir ses dosyasını döndürmeye çalışmıştı. Apple bu yaklaşımı reddetti; geliştiriciye gerçekten duyulabilir audio davranışı sunması veya background-audio yaklaşımını kaldırması gerektiği söylendi. Bu tam olarak OpenContinuity’nin tekrarlamaması gereken hack. citeturn16news37

Dolayısıyla:

**silent audio → hayır**

**fake location → hayır**

**VoIP/background mode’u farklı amaç için kullanma → hayır**

**sonsuz background task chain → hayır**

Teknik olarak bir süre “çalışıyor gibi görünmesi”, App Store-uygun bir mimari olduğu anlamına gelmiyor.

### Local Network ve Bonjour özelinde dikkat edilmesi gerekenler

OpenContinuity’nin iOS versiyonu ilk gerçek LAN erişiminden önce `NSLocalNetworkUsageDescription` açıklaması vermeli ve Bonjour browse/advertise edeceği service type’ları `NSBonjourServices` altında bildirmeli. Apple, local-network privacy izni verilene kadar ilgili trafiğin engellendiğini belgeliyor. citeturn0search4turn23search14

Daha da önemlisi Apple’ın TN3179 açıklamasına göre, uygulama background’dayken Local Network permission hâlâ “undetermined” durumundaysa local-network operasyonu otomatik olarak başarısız olabilir. Bu nedenle onboarding sırasında izni **foreground’da, kullanıcının “Cihaz ara/eşleştir” eylemi sonrasında** tetiklemek doğru tasarım. citeturn0search1

Bonjour’un kendisi background execution entitlement değildir. `NWBrowser`, `NWListener`, Rust socket’i veya mDNS crate’i kullanmanız bunu değiştirmiyor: process suspended olursa listener gelen bağlantıyı işlemez. citeturn23search10

Bu nedenle **Mac→iPhone push modeli yerine iPhone→Mac pull modeli** mevcut iOS için çok daha sağlamdır.

### Custom Keyboard neden cazip ama yanlış MVP olur?

Custom Keyboard extension ilginç çünkü kullanıcı herhangi bir text field’da OpenContinuity keyboard’unu açabilir. Apple’ın resmi belgelerine göre varsayılan custom keyboard’un network erişimi ve containing app ile shared container erişimi yoktur; `RequestsOpenAccess=YES` bunları açar. Keyboard yalnızca host app’in text-input bağlamında çalışır ve text’i `textDocumentProxy` üzerinden yerleştirir. Bazı uygulamalar custom keyboard’ları tamamen devre dışı bırakabilir. citeturn15search0turn9search5

Bu model şu özellik için güzel olabilir:

> “OpenContinuity geçmişinden bir snippet seç ve bulunduğun text field’a yapıştır.”

Ama şu özellik için uygun değildir:

> “Telefonun her yerindeki Copy işlemlerini gizlice izle.”

Ayrıca kullanıcının “Allow Full Access” vermesi, clipboard uygulaması için güven açısından ağır bir onboarding maliyetidir. PastePal gibi uygulamaların keyboard extension kullanması tam da “hızlı tekrar yapıştırma” senaryosu içindir; app listing’i de keyboard ve share extension’larını öne çıkarıyor. citeturn17search15

Ben bunu **MVP’den sonraya** bırakırdım.

### Push notification/service extension da kapıyı açmıyor

Notification Service Extension yalnızca uygun remote notification geldiğinde kısa süreli çalışır ve bildirimin içeriğini değiştirmek üzere tasarlanmıştır. General pasteboard’ı arka planda değiştirebilen sistem servisi değildir. Apple forumlarında extension process’inden `UIPasteboard` erişiminin çalışmadığı örnekler de bulunuyor. citeturn15search1turn3search0

Üstelik OpenContinuity’nin “cloud yok” vaadi açısından APNs tabanlı remote wake sistemi mimari olarak da ters yönde olur: APNs’e gönderim yapacak internet erişimli bir provider gerekir.

Bu kadar altyapı kurup hâlâ clipboard entitlement kazanamıyorsunuz; biraz “asansöre binip aynı katta inmek” olur.

## App Store’a uygun yakın-otomatik tasarım

### iPhone’dan bilgisayara en iyi akış

Mevcut iOS’ta benim birinci tercih edeceğim mekanizma:

```text
Kullanıcı herhangi bir uygulamada Copy yapar
              │
              ▼
     Action Button / Back Tap
     Control Center / Widget
              │
              ▼
       Apple Shortcuts
        Get Clipboard
              │
              ▼
 OpenContinuity.Send App Intent
              │
              ▼
      LAN + pinned mTLS
              │
              ▼
Mac / Windows / Linux clipboard
```

Apple’ın Shortcuts kılavuzu `Get Clipboard` işlemini özellikle widget’tan tamamen çalıştırılabilecek aksiyonlara örnek veriyor: kullanıcı clipboard’a bir şey kopyalayıp Shortcuts widget’ından bir shortcut çalıştırarak içeriği işleyebilir veya başka yere gönderebilir. Aynı dokümanda `Copy to Clipboard` da background/widget-friendly işlem olarak ele alınıyor. citeturn21search1

Dolayısıyla shortcut:

```text
Get Clipboard
    ↓
Send with OpenContinuity
        Device: My Mac
```

olabilir.

OpenContinuity App Intent’i **clipboard’ı kendisi okumamalı**. Parametre olarak Shortcut’tan gelen `String`, image veya `IntentFile` almalı.

Bu ayrım çok önemli:

> Apple’ın system action’ı clipboard’ı okuyor.  
> OpenContinuity yalnızca kullanıcının bilinçli olarak verdiği değeri LAN’a gönderiyor.

Privacy modeli açısından son derece temiz.

### Bu shortcut uygulamayı açmadan gerçekten çalışabilir mi?

**Evet, küçük/normal payload’larda bu tasarım desteklenen bir senaryo.** App Intents foreground veya background çalışma biçimleri tanımlayabiliyor; Apple’ın Shortcuts kılavuzu da `Get Clipboard` gibi işlemlerin widget’tan Shortcuts uygulaması açılmadan tamamlanabileceğini açıkça söylüyor. citeturn6search2turn6search3turn21search1

Ama iki sınır var.

Birincisi, OpenContinuity için Local Network izni daha önce ana uygulama foreground’dayken alınmış olmalı. citeturn0search1

İkincisi, Apple’ın kendisi çok büyük içerikler veya yüksek hacimli Shortcuts işlemleri için Shortcuts uygulamasının açılmasının gerekebileceğini belirtiyor. Bu nedenle shortcut yolu **text ve makul büyüklükte image** için optimize edilmeli; 500 MiB dosya için kullanılmamalı. citeturn21search1

### Bilgisayardan iPhone’a ters yön

Burada daha ilginç bir çözüm var.

Desktop OpenContinuity zaten clipboard’ı sürekli izleyebiliyor. Dolayısıyla desktop tarafında son içeriği kısa süreli “pending latest item” olarak tutabilirsiniz.

Kullanıcı iPhone’da tek hareketle şu shortcut’ı çalıştırır:

```text
OpenContinuity: Fetch Latest from "Office PC"
              ↓
       Copy to Clipboard
        Local Only: ON
```

`Copy to Clipboard` sistem action olduğu için ana OpenContinuity UI’sinin açılması gerekmez; Shortcuts widget türü invocation’larda da uygun bir işlemdir. Apple ayrıca bu action’da **Local Only** seçeneği sunduğunu belgeliyor; bunu etkinleştirmek içeriğin tekrar Apple Handoff üzerinden başka Apple cihazlarına dağılmasını önleyebilir. citeturn21search1

Bu OpenContinuity için özellikle iyi, çünkü olası bir loop’u önlersiniz:

```text
Mac OpenContinuity
   → iPhone
      → Apple Universal Clipboard
         → aynı Mac
            → OpenContinuity tekrar görür
```

Her durumda protocol seviyesinde de:

- `originDeviceId`
- item UUID
- content hash
- sequence number
- TTL

ile loop/dedup yapmanızı öneririm.

### Action Button, Back Tap ve Control Center nasıl kullanılmalı?

**Action Button:** “Send Clipboard” shortcut’ını tek uzun basışa bağlayın. App Intents/Shortcuts ekosistemi Action Button’dan çağrılabiliyor. citeturn8search2turn8search6

**Back Tap:** aynı shortcut çift veya üçlü Back Tap’e atanabilir. Böylece Action Button olmayan cihazlarda bile fiziksel bir hareket sunarsınız. citeturn8search3turn8search7

**Control Center:** iOS 18+ Control Center controls App Intent çağırabiliyor. Fakat custom OpenContinuity Control’ün doğrudan clipboard erişmeye çalışması yerine, kullanıcıya Shortcuts Control üzerinden “Send Clipboard” shortcut’ını seçtirmek bugünkü API’lerle daha güvenli. citeturn8search1turn8search5

**Widget:** Ben MVP’de özel OpenContinuity interactive widget yapmak yerine yine Apple’ın Shortcuts widget’ını öneririm. Çünkü Apple’ın dokümante ettiği `Get Clipboard → process/send` akışı tam olarak burada çalışıyor. citeturn21search1turn8search12

Böylece aynı iki shortcut dört farklı yüzeyden çalışır.

### Share Extension ikinci ana yol olmalı

Kullanıcı özellikle bir şeyi göndermek istiyorsa clipboard’a bile gerek yok:

```text
Safari / Photos / Files / herhangi bir uygulama
          │
          └─ Share
               │
               └─ OpenContinuity
                     │
                     └─ [MacBook]
                           │
                           └─ gönder
```

Share Extension’ın asıl avantajı, arbitrary clipboard okumak yerine host uygulamanın seçilmiş içeriği `NSExtensionItem`/`NSItemProvider` üzerinden açık biçimde size vermesidir. Bu, App Store’un amaçlanan extension modelidir. citeturn6search0turn20search18

UX’i iyi yaparsanız tipik kullanım:

**Share → OpenContinuity**

sonrasında yalnızca tek eşleşmiş/online cihaz varsa transferi doğrudan başlatabilirsiniz.

Birden fazla peer varsa küçük seçim ekranı:

```text
Gönder

● Office PC
○ MacBook
○ Linux Workstation

[ Her zaman Office PC'ye hızlı gönder ]
```

Kullanıcının özellikle opt-in yaptığı “favorite quick send” modunda extension seçildiği anda favorite peer’a gönderme yapılabilir.

Bu, “automatic clipboard” kadar sihirli değil ama gerçek kullanımda yalnızca bir ekstra Share Sheet seçimi.

### 500 MiB dosya/folder transferi için background task farklı ele alınmalı

iOS 26’nın yeni `BGContinuedProcessingTask` API’si clipboard monitoring problemini çözmüyor; fakat OpenContinuity’nin **kullanıcının başlattığı büyük dosya transferi** problemi için oldukça ilgili. Apple bunu foreground’da başlayan, daha uzun sürmesi gereken önemli kullanıcı işlerinin app background’a geçince devam edebilmesi için tanıttı. citeturn0search9turn0search5

Bu nedenle:

**Clipboard text/image sync → App Intent / Share Extension / foreground**

**100–500 MiB dosya transferi → foreground başlat + iOS 26+ continued-processing desteği**

ayrımını yapardım.

Share Extension içinde yüzlerce megabaytlık dosyayı belleğe alıp Rust `Vec<u8>` haline getirmek kesinlikle yanlış mimari olur. `NSItemProvider` üzerinden file representation alın, App Group temporary container’a veya güvenli local temp path’e aktarın ve Rust core’a **path/stream** verin.

### Gerçek anlamda “automatic” yapılabilecek alan

App Store uyumlu olarak bugün şunu sunabilirsiniz:

**OpenContinuity foreground’dayken:**

1. `changeCount` izlenir.
2. Değişmişse pasteboard okunur.
3. Hash/dedup yapılır.
4. Tüm paired desktop’lara otomatik gönderilir.
5. Desktop’tan gelen text/image doğrudan iOS clipboard’a yazılır.

Bu, uygulama açıkken gerçekten tam otomatik çalışabilir.

Uygulama background/suspended olduğunda UI’da bunu dürüstçe:

> **Otomatik senkronizasyon, OpenContinuity açıkken aktiftir.**  
> Uygulama kapalıyken tek dokunuşla göndermek için Action Button veya Shortcut kullanın.

şeklinde anlatmanız App Store metadata’sı açısından da çok daha sağlıklı olur.

“Always-on background clipboard sync” diye pazarlayıp sonra kullanıcıdan silent music açmasını isteyen bir uygulama olmak istemezsiniz. :)

## AB DMA ve yaklaşan çapraz-cihaz pano API’si

Bu bölüm araştırmanın OpenContinuity için stratejik olarak en önemli kısmı.

### iOS 26/27’de Universal Clipboard üçüncü taraflara açılmış mı?

**Bugün itibarıyla doğrudan hayır.**

Apple’ın AB için geliştirdiği interoperability mekanizmaları arasında connected-device background execution, pairing, notification benzeri alanlarda önemli açılımlar var. Apple ayrıca AB’de geliştiricilerin iOS/iPadOS donanım veya yazılım özellikleri için özel interoperability request yapmasına olanak tanıyan resmi bir süreç sunuyor. citeturn17search0turn17search2turn17search6

Ancak iOS 26’nın ilk interoperability değişiklikleri **Universal Clipboard’ı generic üçüncü taraf API’si haline getirmedi**. Bunun en güçlü kanıtı Microsoft’un 2026’da ayrıca clipboard için yeni bir interoperability request yapmak zorunda kalması. Microsoft açıkça public iOS kısıtlarının Windows Phone Link benzeri bir üründe seamless cross-device clipboard’ı engellediğini bildirdi. citeturn19news34

Yani bugün:

```text
DMA APIs
  ├─ bazı connected-accessory imkanları
  ├─ notifications / live activities vb.
  ├─ pairing / transport gelişmeleri
  └─ ❌ genel Universal Clipboard API'si henüz yok
```

### Microsoft talebi tabloyu değiştirdi

Basına yansıyan kayıtlara göre Microsoft’un interoperability request’i:

**Request ID:** `FB22329535`  
**Başvuru:** 25 Mart 2026  
**Apple’ın olumlu proje yanıtı:** 26 Haziran 2026. citeturn18search13

Apple’ın yanıtının yayımlanan metnine göre planlanan çözüm, iOS 26.5’teki Accessory Notifications ve Accessory Live Activities gibi accessory interoperability framework’lerine benzer olacak. Sistem, clipboard’a item kopyalandığında bir geliştirici extension’ını haberdar edecek; extension daha sonra pasteboard item’ını eşleştirilmiş accessory ile paylaşmak için Accessory Transport Extension benzeri mekanizmaları kullanabilecek. Import yönü de tasarımın parçası olarak açıklanıyor. citeturn18news27turn19news34

Bu, OpenContinuity açısından tam olarak eksik iki parçayı çözüyor:

```text
Şu an

Copy
 ↓
iOS clipboard
 ↓
?????????           ← app uyandırılmıyor
 ↓
OpenContinuity


Planlanan DMA yolu

Copy
 ↓
iOS clipboard
 ↓
system notification
 ↓
OpenContinuity accessory extension
 ↓
authorized paired PC
```

Apple çalışmanın 2027 sonbaharında tamamlanmasını, ardından developer beta sunulmasını hedefliyor. Bu nedenle **iOS 27 üretim SDK’sında bugün kullanabileceğiniz bir API olarak düşünülmemeli.** citeturn19news34turn18news27

### OpenContinuity bu haktan yararlanabilir mi?

**Potansiyel olarak evet ve bu konuda proaktif davranmanızı öneririm.**

Microsoft’un hedefi Windows PC olduğu için “accessory” kelimesinin yalnızca kulaklık/saat gibi donanımları ifade etmediği görülüyor. Bu, OpenContinuity’nin Windows peer’ı açısından özellikle güçlü bir emsal. Mac/Linux desktop yazılımlarının da aynı gelecek API’sinde “paired accessory endpoint” olarak kabul edilip edilmeyeceği ise public SDK çıkmadan kesin söylenemez. Bu ikinci kısım benim kaynaklardan çıkardığım bir çıkarımdır. citeturn19news34turn17search0

Apple’ın resmi EU interoperability request formunu kullanarak OpenContinuity adına aynı use-case’i kayda geçirirdim:

> Cross-device pasteboard interoperability between an iPhone/iPad and paired macOS, Windows and Linux computers, including system notification of pasteboard changes while the companion app is not foreground, and the ability to import an item into the pasteboard from an authenticated paired computer.

Başvuruda Microsoft’un kamuya yansıyan `FB22329535` talebine de referans verilebilir. Apple, AB’deki app geliştiricilerinden gelen interoperability taleplerini değerlendirip çoğu başvuru için uygunluğa ilişkin ilk değerlendirmeyi 20 iş günü içinde ilettiğini söylüyor. citeturn17search0

Bu API geldiğinde OpenContinuity’nin mimarisini baştan yazmamak için bugünden:

```text
Clipboard source
      ↓
Sync Core
      ↓
Transport interface
```

şeklinde ayırın.

Bugünkü source:

```text
Foreground UIPasteboard / Shortcut / Share Extension
```

gelecekte:

```text
EU Cross-Device Clipboard Extension
```

olabilsin.

### Wi‑Fi Aware de izlenmeye değer

Apple’ın güncel Wi‑Fi Aware dokümanları, uygulamanın AccessorySetupKit veya DeviceDiscoveryUI ile eşleştirilmiş Wi‑Fi Aware cihazlara foreground ve background durumlarında bağlanabileceğini belirtiyor. Bu çok ilginç bir transport gelişmesi. citeturn15search4turn17search14

Ancak iki nedenle MVP’nin merkezine koymazdım:

Birincisi, bu **clipboard değişim entitlement’ı değildir**.

İkincisi, OpenContinuity’nin Windows/macOS/Linux tarafında Wi‑Fi Aware desteği ve donanım uyumluluğu, normal LAN+mDNS’e kıyasla çok daha karmaşık olacaktır.

Yine de gelecekte “iPhone suspended iken paired desktop ile güvenilir bağlantı kurabilme” açısından takip edilecek en önemli Apple networking API’lerinden biri.

## Rakipler, kullanıcı şikâyetleri ve fırsatlar

### Rakipler aslında iOS sınırlamasını nasıl dolaşıyor?

En ilginç bulgu, ciddi rakiplerin büyük bölümünün bu sınırlamayı “hack” etmemesi. Bunun yerine UX’i sınırlamanın etrafında kuruyorlar.

| Ürün | iOS yaklaşımı / gözlem | OpenContinuity için ders |
|---|---|---|
| **Paste** | Mac/iPhone/iPad clipboard history ve sync sunuyor; fakat kendi yardım sayfası iOS background’unda clipboard otomatik yakalayamadığını açıkça söylüyor. citeturn16search8turn19search29 | Kullanıcıya OS limitini açıkça anlat; extension/keyboard/manual capture ile tamamla. |
| **PastePal** | iCloud sync, custom keyboard, iMessage ve Share extensions sunuyor; güncel listing ayrıca “No automatic tracking” vurgusu yapıyor. citeturn17search15 | Keyboard ve Share extension iOS’un izin verdiği user-driven yollar. |
| **ClipSync** | Listing “automatically saves” gibi güçlü ifade kullanıyor ama açıklama içeriğin **ClipSync açıldığında** yakalandığını söylüyor. citeturn16search19 | “Automatic” pazarlamasına dikkat; iOS gerçeğini gizlemeyin. |
| **KDE Connect iOS** | App Store “shared clipboard” özelliği sunuyor. Kullanıcı raporlarında iOS yönünde clipboard’ın manuel gönderilmesi veya sync’in tek yönlü davranması gibi sorunlar var. citeturn19search11turn19search7turn19search19 | LAN-first ve open-source olması yakın rakip; iOS lifecycle UX’i kritik. |
| **LocalSend** | Çok iyi LAN file sharing ürünü; kullanıcılar alınan içeriğin ekstra “Copy” tıklaması olmadan doğrudan clipboard’a gitmesini istiyor. citeturn19search4turn22search3 | “Receive → clipboard” OpenContinuity’nin güçlü ayırt edicisi olabilir. |
| **Clipt / OnePlus** | Clipboard/file transfer için Google Drive tabanlı ara depolama kullandı ve 2023’te kapatıldı. citeturn16search6turn16search29 | No-cloud LAN-only kalıcılık ve bağımsızlık önemli bir farklılaştırıcı. |
| **Clip Cloud** | Cross-platform clipboard’u Google Cloud Message/cloud altyapısıyla yapıyor. citeturn16search18 | Siz privacy/local-first segmentini hedefleyebilirsiniz. |
| **Cross Paste** | Ağustos 2026 App Store listing’i iPhone/iPad/Mac/Windows/web, QR pairing, no-account ve E2E encryption iddialarını öne çıkarıyor. citeturn16search30 | QR onboarding artık kullanıcı beklentisi olmalı. |
| **Octoclip** | iOS’ta clipboard history ve “Nearby Sync” üzerinde aktif geliştirme yapıyor. citeturn16search25 | Yakın cihaz + history birleşimi rekabet alanı. |
| **Pastebot** | Güncel Pastebot 3 macOS odaklı. Eski iOS Pastebot ise background audio hilesinin tarihsel örneği ve Apple rejection’ıyla biliniyor. citeturn16search23turn16news37 | Silent audio gibi workaround’lardan uzak durun. |
| **Windows Phone Link + iPhone** | Bugün iPhone clipboard sync sunamaması Microsoft’u DMA interoperability request yapmaya götürdü. citeturn19news34turn18news27 | “Microsoft bile public API ile yapamıyor” mevcut kısıtın güçlü doğrulaması. |
| **Pushbullet** | Tarihsel iPhone app’i cihazlar arası içerik gönderimini cloud/push ile yaptı. citeturn19search17 | Cloud mesajlaşması explicit send’i kolaylaştırır ama background pasteboard hakkı yaratmaz. |
| **Snapdrop tipi web/PWA araçları** | Web/share yaklaşımı explicit user transferine dayanır; native iOS background pasteboard entitlement’ı kazandırmaz. | Share-sheet-benzeri UX ile clipboard watcher problemini atlatır, çözmez. |

### Kullanıcıların gerçekten şikâyet ettiği noktalar

Clipboard-manager tartışmalarında kullanıcılar iOS’ta uygulamayı yeniden açma veya keyboard monitoring’i tekrar devreye alma ihtiyacından şikâyet ediyor. PasteNow hakkında bir kullanıcı örneğinde, mevcut clipboard’ın history’ye alınması için uygulamanın açılması gerektiği belirtiliyor; PastePal kullanıcıları da keyboard tabanlı monitoring’in sürekliliğinden yakınmış. citeturn22search4turn16search5

Bu nedenle OpenContinuity’de **“uygulamayı açık tut”u ana UX yapmamak**, bunun yerine Action Button/Back Tap/Shortcut çözümünü birinci sınıf özellik yapmak çok önemli.

LocalSend kullanıcılarında başka bir ortak talep de var: gelen şeyi diske kaydet → bul → Copy yapmak yerine doğrudan clipboard’a almak. Ağustos 2026 tarihli bir feature request, gönderilen clipboard içeriğinin bilgisayarda ekstra pencere/Copy butonu gerektirmeden doğrudan clipboard’a geçmesini istiyor; başka bir issue image/media transferlerinin depolama yerine clipboard’a doğrudan aktarılmasını istiyor. citeturn19search4turn22search3

Bu, OpenContinuity’nin ürün mesajı açısından güçlü:

> **LocalSend dosya gönderir. OpenContinuity çalışma bağlamınızı taşır.**

### Files ve Photos davranışı kullanıcı için şaşırtıcı olabiliyor

LocalSend’in iOS issue tracker’ı burada iyi bir UX laboratuvarı.

Kullanıcılar:

- transfer edilen dosyanın Files içinde nereye gittiğini bulamamaktan,
- media’nın Files yerine Photos’a gitmesinden,
- yüzlerce fotoğrafın camera roll’a karışmasından,
- belirli bir albüme kaydedememekten,
- transfer sırasının Photos içinde korunmamasından

şikâyet ediyor. citeturn22search1turn22search9turn22search15

Bu nedenle OpenContinuity için “image geldi → otomatik Photos’a at” davranışını varsayılan yapmazdım.

Daha iyi model:

```text
Gelen PNG

[ Panoya Kopyala ]      ← varsayılan clipboard transfer
[ Dosyalara Kaydet ]
[ Fotoğraflara Kaydet ]
```

Dosya transferi olarak gelen medya için kullanıcı ayarı:

```text
Alınan fotoğraf/video hedefi

○ OpenContinuity Inbox
○ Photos
● Her seferinde sor
```

Photos seçilirse ileride özel bir **OpenContinuity** albümü seçeneği eklemek de mantıklı.

### Büyük dosya ve toplu medya transferi ayrı test edilmeli

LocalSend kullanıcıları iPhone’dan birkaç GB büyüklüğünde transferlerde crash ve yüzlerce fotoğrafın Share Sheet üzerinden hazırlanmasında başarısızlık bildirmiş. Ayrıca yeni kurulumda Photos izninin doğru zamanda istenmemesi nedeniyle receive akışının bozulduğu raporlar var. citeturn22search7turn22search11turn22search13

OpenContinuity’nin 500 MiB üst sınırı burada avantaj: kapsamınız zaten belli.

Ama şunları özellikle test edin:

| Test | Beklenen davranış |
|---|---|
| 500 MiB tek dosya | RAM’e tamamen yüklemeden stream |
| 500 MiB folder | path traversal korumasıyla stream/archive |
| 300–500 PNG | extension memory pressure olmadan |
| Photos → Share → OpenContinuity | mümkün olduğunca original representation |
| Files provider/iCloud Drive file | security-scoped access doğru tutulmalı |
| ekran kilitlenmesi | transfer devam/temiz iptal |
| foreground → background | iOS 26 continued processing |
| Wi‑Fi değişimi | kontrollü reconnect/resume |
| peer kaybolması | temp data temizliği + retry UX |
| disk alanı yetersiz | transfer başlamadan mümkünse kontrol |

Share Sheet üzerinden Photos içeriğinin transcoding/preparing davranışı da ayrıca test edilmeli; LocalSend issue’larında Share üzerinden seçilen media’nın original file’dan farklı hazırlanabildiği görülüyor. citeturn22search19

### App Review’de gerçekten reddedilmiş workaround’lar

Kamuya açık, güvenilir örnek olarak en somut clipboard örneği eski Pastebot’un background-audio numarası. Apple bunu kabul etmedi. citeturn16news37

2026’da da “audio background mode var ama reviewer persistent audio gerektiren fonksiyon göremiyor” gerekçesiyle 2.5.4 rejection örnekleri devam ediyor. citeturn10search14

Buna karşılık araştırmada Paste, PastePal, KDE Connect veya LocalSend’in “clipboard hilesi nedeniyle reddedildiğine” dair güvenilir bir kamu kaydı bulmadım. Bu uygulamaların izlediği yol tam tersine çoğunlukla platform kısıtına uyup Share Extension, keyboard, foreground veya explicit user action kullanmak.

## Önerilen ürün kapsamı ve UX

### MVP’de olması gerekenler

Önceliği şöyle verirdim:

| Öncelik | Özellik | Neden |
|---|---|---|
| **P0** | QR pairing + 6 haneli fallback | Mobilde pairing friction’ını dramatik azaltır; Cross Paste gibi güncel rakipler de QR modelini kullanıyor. citeturn16search30 |
| **P0** | Foreground text + PNG automatic clipboard sync | Masaüstündeki mevcut davranışın iOS’un izin verdiği kısmını korur. |
| **P0** | Share Extension: text/link/image/file/folder | iPhone→desktop için en doğal App Store-uyumlu yol. |
| **P0** | App Intent: `Send Content` | Shortcuts entegrasyonunun temel primitive’i. |
| **P0** | App Intent: `Fetch Latest` | Mac/Windows/Linux→iPhone bir-hareket clipboard için temel. |
| **P0** | Hazır “Send Clipboard” Shortcut onboarding’i | `Get Clipboard → Send`. |
| **P0** | Hazır “Fetch Clipboard” Shortcut onboarding’i | `Fetch Latest → Copy to Clipboard`. |
| **P0** | Action Button / Back Tap kurulum rehberi | Automatic’e en yakın his. |
| **P0** | Foreground inbound clipboard write | OpenContinuity açıkken gerçek iki yönlü auto-sync. |
| **P0** | Received Files Inbox | Dosyanın nereye gittiği hiçbir zaman belirsiz olmasın. |
| **P0** | Save to Files / Save to Photos açık ayrımı | LocalSend’de görülen kullanıcı karışıklığını önler. citeturn22search1turn22search9 |
| **P0** | Device revoke + certificate fingerprint | Mevcut mTLS trust modelinizi mobilde görünür kılar. |
| **P0** | Transfer history | “Ne gönderildi, hangi cihazdan geldi?” sorununu çözer. |
| **P0** | 500 MiB streaming transfer | Rust/Swift boundary’de büyük `Data` copy’si yapmadan. |
| **P0** | `BGContinuedProcessingTask` iOS 26+ | Büyük kullanıcı-başlatımlı dosya transferinin background’a geçebilmesi için. citeturn0search9 |

MVP’de **“Clipboard History” yerine “Transfer History”** adını kullanmanızı öneririm.

Çünkü iOS background’daki her Copy’yi yakalayamayacağınız için “Clipboard History” kullanıcıya eksiksiz bir tarihçe vaat eder. “Transfer History” ise gerçekte neyi kaydettiğinizi dürüstçe anlatır.

### Pairing: QR kesinlikle ana yöntem olmalı

Mevcut altı haneli pairing code’u kaldırmayın; ama iPhone’da primary flow QR olmalı.

Önerilen akış:

```text
Desktop

OpenContinuity
  → Add Device
  → [ QR CODE ]

      "OpenContinuity Pairing"
      Office PC
      Expires in 2:00


iPhone

OpenContinuity
  → Pair Device
  → Kamera açılır
  → QR tara
  → "Office PC ile eşleş?"
  → [ Eşleştir ]
```

QR payload’a mümkünse:

- protocol version,
- ephemeral public key veya certificate fingerprint,
- random one-time pairing nonce,
- opaque device identifier,
- kısa expiration,
- gerekirse discovery hint

koyun.

**Altı hane tek başına authentication secret olmamalı.** Altı hane yalnızca yaklaşık bir milyon kombinasyon demektir; onu SAS/user-verification katmanı olarak değerlendirin. Asıl trust cryptographic pairing materyali ve pinned cert üzerinden kurulmalı.

Fallback:

```text
QR tarayamıyor musunuz?
[ 6 haneli kod ile eşleştir ]
```

Bu hem mevcut desktop protocol’ünüzü korur hem mobil UX’i iyileştirir.

### Local Network iznini ilk açılışta patlatmayın

Kötü onboarding:

```text
App açıldı
→ "OpenContinuity would like to find devices on your local network"
```

Kullanıcı daha uygulamanın ne yaptığını anlamamış oluyor.

Daha iyi:

```text
"Cihazlarınızı doğrudan yerel ağda bulacağız.
Verileriniz internete gönderilmez."

[ Cihaz Eşleştir ]
       ↓
iOS Local Network prompt
```

Apple’ın local-network privacy modeli de erişimin kullanıcı bağlamında başlatılmasını gerektiriyor; background’da henüz verilmemiş izni istemeye çalışmak başarısız olabiliyor. citeturn0search1turn0search4

### iPhone → desktop için üç katmanlı UX

**En hızlı:** Action Button / Back Tap shortcut.

```text
Copy
→ Action Button
→ done
```

**Her uygulamada:** Share Sheet.

```text
Share
→ OpenContinuity
→ done
```

**OpenContinuity açıksa:** tam automatic.

```text
Copy
→ no further interaction
```

Kullanıcı hangisini tercih ederse ona geçebilsin.

### Desktop → iPhone için “pending clipboard” tasarımı

Bu bence iOS sürümünün en önemli özgün UX fikri olabilir.

Desktop her yeni clipboard item’ında:

```text
latest_syncable_item = item
```

tutsun.

iPhone shortcut:

```text
Fetch Latest
→ Copy to Clipboard
```

Peer offline ise:

> Office PC’ye ulaşılamadı.

Peer online ama item değişmemişse:

> Pano zaten güncel.

Yeni item varsa:

> Office PC’den kopyalandı.

Bu modelde desktop’ın iPhone’a inbound bağlantı kurması gerekmiyor. iPhone kullanıcı eylemiyle kısa outbound connection kuruyor; iOS’un background networking modeline çok daha uygun.

İsterseniz desktop’ta yalnızca tek latest item değil, küçük bir **pending ring buffer** tutabilirsiniz:

```text
Office PC
──────────────
⌘  En son pano
1m URL
3m PNG
5m text
```

iPhone tarafında “Fetch Latest” bir hareket; OpenContinuity uygulaması açıldığında geçmiş item’lara erişim.

### Files ve Photos

**Non-media dosyalar:** varsayılan `OpenContinuity Inbox`.

Kullanıcıya:

- Preview
- Share
- Save to Files
- Delete

sunun.

**PNG/image clipboard item:** doğrudan clipboard içeriği olarak ele alın. Otomatik Photos’a atmayın.

**Explicit image/file transfer:** kullanıcı tercihine göre Files veya Photos.

LocalSend kullanıcılarının camera roll’a toplu medya düşmesi ve albüm ihtiyacı hakkındaki istekleri, bu ayrımın gerçekten değerli olduğunu gösteriyor. citeturn22search9turn22search15

Folder için transfer sırasında klasör yapısını koruyun. Protocol zaten folders destekliyorsa iOS sandbox içine aynı tree’yi çıkarabilirsiniz; archive kullanıyorsanız absolute path, `..`, symlink ve zip-slip kontrollerini kesinlikle core katmanda yapın.

### Güvenlik modelinde iOS’a özel eklemeler

Mevcut modeliniz zaten oldukça iyi bir başlangıç:

```text
no cloud
no account
mTLS
pinned certificates
local network
paired devices
```

iOS’ta bunun üzerine şu UX’i eklerdim:

```text
Trusted Devices

Office PC
Windows 11
Last seen: şimdi
Fingerprint: A4:8C:…
[ Disconnect ]

MacBook
macOS
Last seen: 2 saat önce
[ Disconnect ]
```

History için varsayılanı mümkün olduğunca privacy-first tutun:

```text
Transfer History
Retention: 24 saat

Sensitive Content
[ ] Text payload'larını kalıcı sakla
[x] Transfer metadata'sını sakla
```

Clipboard password, API key, OTP, recovery phrase gibi şeyleri taşıyabileceğinden “sonsuz clipboard history” OpenContinuity’nin varsayılan davranışı olmamalı.

Bence daha güçlü ürün mesajı:

> **Clipboard’unuzu arşivlemiyoruz; cihazlarınız arasında taşıyoruz.**

History isteyen kullanıcı ayrıca açabilir.

### Daha sonraki sürümler

MVP oturduktan sonra şu sırayı izlerdim:

**Custom Keyboard:** OpenContinuity history/snippet’lerinden mevcut text field’a yapıştırmak için. Background monitor olmak için değil.

**Native Control Center control:** “Fetch Latest” gibi clipboard okuması gerektirmeyen veya App Intent üzerinden çalışan komutlar için.

**Interactive widget:** online peer’lar, son transfer, “Fetch” butonu.

**Live Activity:** özellikle yüzlerce megabaytlık aktif file/folder transferinde progress göstermek için.

**Searchable/pinned history:** ancak kullanıcı talebi doğrulandıktan sonra.

**Per-device rules:** örneğin “Office PC’den gelen clipboard otomatik uygulanmasın”, “PNG sync kapalı”, “500 KB üzerindeki images manuel”.

**Temporary clipboard mode:** “Bu item 2 dakika sonra silinsin.” Shortcuts’ın `Copy to Clipboard` action’ı expiration kavramını destekliyor. citeturn21search1

**EU future clipboard extension:** çıktığı anda öncelik P0’a dönüşmeli.

## Önerilen iOS mimarisi

### Karar: SwiftUI shell + Rust core + UniFFI

Üç seçeneği şöyle değerlendiriyorum:

| Yaklaşım | Native iOS entegrasyonu | Mevcut Rust reuse | Share/AppIntent/Widget kolaylığı | Gelecek DMA API’lerine uyum | Tavsiye |
|---|---:|---:|---:|---:|---|
| **SwiftUI + Rust/UniFFI** | Çok yüksek | Çok yüksek | Çok yüksek | Çok yüksek | **En iyi seçenek** |
| **Tauri 2 mobile + Swift plugins + Rust** | Orta/yüksek | Çok yüksek | Orta | Orta/yüksek | Yapılabilir ama gereksiz katman |
| **Tamamen Swift rewrite** | Çok yüksek | Çok düşük | Çok yüksek | Çok yüksek | Protocol/crypto duplication nedeniyle önermiyorum |

Tauri 2 gerçekten iOS destekliyor; mobil plugin’lerde native Swift sınıfları kullanabiliyor ve Swift kodunu Rust/JavaScript tarafına açabiliyor. Yani “Tauri iOS’ta olmaz” gibi bir problem yok. citeturn20search0turn20search4turn20search8

Problem şu ki OpenContinuity iOS’un karmaşıklığı burada:

```text
HTML/CSS UI
```

değil.

Asıl karmaşıklık:

```text
UIPasteboard
Share Extension
App Intents
Shortcuts
WidgetKit / Controls
Photos
Files
Local Network permission
Bonjour
BGContinuedProcessingTask
AccessorySetupKit
gelecekte DMA clipboard extension
```

Bunların büyük bölümü zaten Swift/Xcode target dünyasında çözülecek.

Tauri kullanırsanız sonuç büyük ihtimalle:

```text
Web UI
  ↕
Tauri Swift plugin
  ↕
Rust
```

ve ayrıca:

```text
Share Extension ─ Swift
App Intent ────── Swift
Widget ────────── Swift
Control ───────── Swift
```

olacak.

Yani UI paylaşımı kazanırken sistem entegrasyonunu basitleştirmiyorsunuz.

Native SwiftUI + UniFFI ile:

```text
SwiftUI
  ↕
UniFFI
  ↕
Rust
```

kalıyor.

UniFFI doğrudan Rust library’den Swift bindings üretmek üzere tasarlanmış ve Xcode projesine static library/bindings şeklinde entegre edilebiliyor. citeturn20search1turn20search5

### Önerilen katmanlar

```text
┌─────────────────────────────────────────────┐
│                SwiftUI App                  │
│                                             │
│  Pairing       Devices       Inbox          │
│  History       Settings      Permissions    │
└───────────────────┬─────────────────────────┘
                    │
       ┌────────────┼─────────────┐
       │            │             │
       ▼            ▼             ▼
 UIPasteboard   App Intents   Photos / Files
 foreground     Shortcuts
 watcher
       │            │
       └───────┬────┘
               ▼
┌─────────────────────────────────────────────┐
│              Swift iOS Layer                │
│                                             │
│ lifecycle / permissions / extensions        │
│ App Group / Keychain / NW integration       │
└───────────────────┬─────────────────────────┘
                    │ UniFFI
                    ▼
┌─────────────────────────────────────────────┐
│         opencontinuity-core (Rust)          │
│                                             │
│ protocol                                    │
│ mTLS + cert pinning                         │
│ pairing state machine                       │
│ device/group model                          │
│ framing                                     │
│ hashing + dedup                             │
│ file metadata/path validation               │
│ transfer state                              │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
             LAN transport
        Mac / Win / Linux peers
```

Share Extension ayrıca:

```text
Other app
  │
  ▼
Share Extension (Swift)
  │
  ├─ NSItemProvider
  ├─ target chooser
  │
  ▼
Rust one-shot send
  │
  ▼
LAN peer
```

şeklinde olabilir.

Apple, uygulama ile extension arasında veri paylaşılması gerekiyorsa App Group kullanılmasını öneriyor. citeturn20search3turn20search7turn20search11

### Rust crate’i biraz yeniden sınırlandırmak faydalı olur

Uzun vadede Rust engine’i üç mantıksal parçaya ayırmanızı öneririm:

```text
opencontinuity-protocol
  crypto
  wire format
  pairing
  item model
  dedup
  group trust


opencontinuity-transport
  desktop sockets
  mDNS
  streaming


opencontinuity-engine
  orchestration
```

iOS’ta `protocol` katmanı mümkün olduğunca platform bağımsız olmalı.

Transport için iki seçenek açık kalmalı:

```text
Bugün
Rust LAN transport / Network.framework adapter

Gelecekte AB
Accessory Transport Extension
        ↓
aynı Rust protocol core
```

Böylece 2027+ DMA API’si geldiğinde protocol/crypto’nuzu yeniden yazmazsınız.

### UniFFI sınırından büyük payload geçirmeyin

Şunu yapmayın:

```swift
let data = try Data(contentsOf: 500MBFile)
rustSend(data)
```

Çünkü Swift `Data` → FFI → Rust `Vec<u8>` arasında büyük kopyalar ve memory pressure üretebilirsiniz.

Bunun yerine:

```text
Swift
NSItemProvider
   ↓
temporary / App Group file
   ↓
path + metadata
   ↓
Rust
stream file
```

şeklinde gidin.

Örneğin API:

```text
sendText(peer, text)

sendImageFile(peer, path, mimeType)

sendFile(peer, path, displayName)

sendDirectory(peer, rootPath)

fetchLatest(peer)
```

olsun.

Swift tarafı security-scoped URL veya `NSItemProvider` yaşam döngüsünü yönetir; Rust tarafı bytes’ı stream eder.

### App Intent API yüzeyi küçük tutulmalı

İlk sürüm için iki temel intent yeterli:

```text
SendContentIntent
  input
  destination


FetchLatestIntent
  source
  returns content
```

Clipboard-specific logic custom intent içinde olmamalı.

Böylece Shortcuts:

```text
Get Clipboard
→ SendContentIntent
```

ve:

```text
FetchLatestIntent
→ Copy to Clipboard
```

şeklinde compose edilir.

Bu tasarım Apple’ın privacy modeline çok doğal oturuyor.

### “App Intent ile LAN isteği her zaman çalışır” varsayımı yapmayın

App Intent background’da çalışabilir, fakat onu persistent network daemon gibi düşünmemek gerekir. Intent:

```text
connect
authenticate
small request/response
disconnect
```

modelinde kısa ve deterministik olmalı. Apple’ın App Intents mimarisi foreground ve background execution’ı destekliyor, ancak generic persistent sockets için ayrı bir yetki sağlamıyor. citeturn6search2turn6search7turn23search10

Bu nedenle `Fetch Latest` protocol mesajı ideal olarak çok hafif olmalı:

```text
HELLO
AUTH
GET_LATEST_METADATA
GET_PAYLOAD
BYE
```

Text ve PNG için çok iyi.

500 MiB transfer için uygun değil.

### Discovery latency’sini azaltmak için son endpoint’i cache’leyin

Shortcut invoked olduğunda baştan:

```text
mDNS browse
→ bekle
→ peer bul
→ TLS
```

kullanıcıya yavaş gelebilir.

Pairing sırasında:

```text
device ID
certificate fingerprint
last known address(es)
Bonjour service ID
```

saklayabilirsiniz.

Intent:

```text
last-known endpoint'e bağlan
        │
   başarılı?
   /       \
 evet      hayır
  │          │
 send      kısa Bonjour resolve
```

yapsın.

Pinned certificate nedeniyle eski IP başka cihaza geçmiş olsa bile TLS identity verification başarısız olacağı için yanlış peer’a veri göndermemelisiniz.

### iOS 27 yaşam döngüsü açısından SwiftUI ayrıca avantajlı

Apple WWDC25’te iOS 26’dan sonraki SDK döneminde UIKit uygulamalarının UIScene lifecycle kullanması gerekeceğini duyurdu. SwiftUI application lifecycle ile yeni bir proje açmak bu gereksinimi doğal olarak karşılıyor. citeturn21search2

Bu da 2026’da sıfırdan başlayan OpenContinuity iOS için legacy UIKit/AppDelegate mimarisine yatırım yapmamak adına başka bir neden.

### Ben olsam ilk release’i tam olarak şöyle çıkarırdım

**Ana ekran**

```text
OpenContinuity

Connected
────────────
● Office PC
● MacBook

Quick Actions
────────────
[ Send Clipboard ]
[ Fetch Clipboard ]

Recent Transfers
────────────
Office PC  ←  "meeting at 15:00"
MacBook    →  image.png
```

`Send Clipboard` foreground’da explicit paste ile çalışır.

`Fetch Clipboard` latest item’ı alır ve clipboard’a yazar.

İlk onboarding sonunda:

```text
Daha hızlı kullanın

[ Add Send Clipboard Shortcut ]
Action Button, Back Tap ve Control Center'dan
tek hareketle gönderebilirsiniz.

[ Add Fetch Clipboard Shortcut ]
```

Share Extension ilk günden gelir.

QR pairing ilk günden gelir.

Text + PNG + Files/Folders ilk günden gelir.

Background auto-sync konusunda gerçek dışı vaat verilmez.

### Stratejik yol haritası

Bugün için hedef:

> **“iOS’un izin verdiği en az dokunuşlu cross-platform clipboard.”**

olmalı; “Universal Clipboard’ın birebir background klonu” değil.

Bunun en iyi mevcut kombinasyonu:

```text
Foreground                → tam otomatik
Share Extension           → explicit içerik, 1–2 tap
Action Button + Shortcut  → clipboard, 1 hareket
Back Tap + Shortcut       → clipboard, 1 hareket
Control Center Shortcut   → clipboard, 1 tap
Mac→iPhone Fetch Shortcut → clipboard, 1 hareket
```

Apple’ın AB için geliştirmeyi kabul ettiği yeni paired-accessory clipboard API’si yayınlandığında ise ürünün AB sürümünde gerçek hedef:

```text
Copy on iPhone
        ↓
system wakes OpenContinuity extension
        ↓
paired Mac / Windows / Linux
        ↓
paste
```

olabilir. Microsoft’un 2026 DMA talebi sonucunda Apple’ın tam olarak bu tür bir extension üzerinde çalışıyor olması, OpenContinuity açısından bence araştırmanın en olumlu bulgusu. Bugün yapılması gereken, MVP’yi hack’lerle App Review sınırına sürmek değil; **Swift-native extension mimarisini ve Rust protocol core’unu o gelecek API’ye takılabilecek şekilde ayırmak**. citeturn19news34turn18news27turn17search0