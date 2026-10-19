# OpenContinuity — proje notları

Mac ↔ Windows ↔ Linux (X11) yerel ağ pano senkronizasyonu (Rust). Ürün kapsamı ve kararlar: `OpenContinuity_Proje_Plani.md`. Belgeler `docs/` altında.

## Komutlar

```sh
source "$HOME/.cargo/env"
cargo build                                        # macOS
cargo build --target x86_64-pc-windows-gnu         # Windows .exe (mingw-w64, .cargo/config.toml)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --target x86_64-unknown-linux-gnu -p oc-clipboard --all-targets -- -D warnings   # Linux pano adaptörü (yerelde yalnız bu crate; engine `ring` C derleyicisi ister → CI)
cd apps/desktop/src-tauri && cargo tauri build --bundles app,dmg                          # macOS paketi (yerel, imzasız; yayın CI'dan)
# Windows installer: msvc + cargo-xwin (gnu hedefi WebView2Loader.dll'e bağımlı kalıyor, kullanıcıda 'bulunamadı' hatası verdi)
cd apps/desktop/src-tauri && PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
# iOS: Rust çekirdeği + UniFFI Swift bağları → apps/ios/Core (xcode-select CLT'de; betik DEVELOPER_DIR verir)
scripts/build-ios-core.sh [debug]
cd apps/ios && xcodegen && DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -project OpenContinuity.xcodeproj -scheme OpenContinuity -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' -derivedDataPath build CODE_SIGNING_ALLOWED=NO build
OC_CONFIG_DIR=<dizin> OC_PORT=47810 cargo run -p oc-engine --example fake_peer   # sistem panosuna dokunmayan test eşi (stdin: pair, e, text …, png …, files …)
```

`cargo fmt` çalıştırma: depo varsayılan rustfmt genişliğinde değil, tüm dosyaları yeniden biçimliyor.

## Yapı

- `crates/engine` → senkron motoru (eşleştirme + gruptaki eşlerle senkron döngüsü, olay/komut API'si); CLI ve masaüstü bunu kullanır
- `apps/cli` → `oc` binary'si (teşhis aracı)
- `apps/desktop` → Tauri 2 uygulaması (`src-tauri/` Rust, `ui/` derleme adımsız HTML/JS)
- `apps/ios` → SwiftUI uygulaması (XcodeGen `project.yml`; `.xcodeproj` ve `Core/` üretilir, depoda yok). Motoru `crates/ffi` (UniFFI) ile kullanır; pano `ClipboardSource::Port`, keşif Swift Bonjour'u → `DiscoveryMode::External` (iOS'ta ham multicast için Apple yetkisi gerekir). Simülatör portu 47801 (Mac'teki uygulama 47800'ü tutar).
- `crates/<ad>` → kütüphaneler; her yeni crate `Cargo.toml` `members` listesine eklenir (glob boş eşleşirse cargo hata verir)

## Kurallar

- Pano içeriği hiçbir log, test çıktısı veya hata mesajına yazılmaz; yalnız uzunluk + hash öneki.
- Kriptografi kendimiz yazılmaz (rustls/ring, rcgen, sha2).
- Gerçek cihaz testi yapılmadıysa `docs/test-plan.md`'de işaretlenmez.
- Dosya başına 300 satırı geçme.

## Durum (2026-10-19 gece, oturum sonu)

Depo https://github.com/behcetozcann/opencontinuity (herkese açık, PolyForm Noncommercial). Yayında: **v0.1.11** (2026-10-19 21:37; yeni simge "satırlı kâğıt" — uygulama, iOS, tepsi; OC_CONFIG_DIR'li deneme örneği artık girişte başlatma kaydına dokunmuyor). Önceki: v0.1.10 (2026-10-19; `main` = etiket v0.1.10, ec3d0ef; protokol v6: metin + görsel + dosya/klasör + grup). 0.1.10: gelen dosya/görselde sesli bildirim (`notifications.rs`, tauri-plugin-notification, macOS "Glass"; metin sessiz; Ayarlar › Genel'den kapatılır; gerçek Mac'te kullanıcı onaylı) + `ios` dalı `main`'e katıldı. 1.0 için konuşulan ölçüt: gerçek cihaz testleri (M, I, Linux paketi), protokol v7 sabitlenmesi, iOS App Store, Windows kod imzası, gerçek simge. macOS paketi Developer ID ile imzalı (Team LKD3F8VA6H, kimlik `org.opencontinuity.desktop`) ve noterlikli. 0.1.8 numarası yayınlanmadan atlandı. Windows paketi imzasız (SmartScreen "Yine de çalıştır" uyarısı; öneri Azure Trusted Signing, kullanıcı karar vermedi).

0.1.9 gerçek cihazda (Mac + Windows + DentOs Linux Mint) L1–L7 geçti; "cihaz eklerken bağlantılar kopmaz" düzeltmesi de gerçek cihazda doğrulandı (M7). Test paketlerine bundan sonra ön sürüm numarası verilecek (ör. `0.1.10-test.1`), aynı numara güncelleyiciyle gelmiyor.

Faz 2 yapılanlar (hepsi gerçek cihazda kullanıcı onaylı):
- Görsel panosu (0.1.3, v4): PNG, ≤ 32 MiB, 256 KiB parçalar. macOS `public.png`/TIFF→PNG; Windows `PNG` biçimi (IEND'e göre kırpılır) / CF_DIB→PNG, yazarken PNG + CF_DIB. Metin + görsel birlikteyse metin gider.
- Dosya/klasör (0.1.4, v5): ≤ 500 MiB, 5.000 girdi; `engine/src/files.rs` (toplama, diske yazma, kabul/budama), `writer.rs` (ayrı yazma görevi; ping araya girer, yeni kopya süren aktarımı `TransferCancel` ile bırakır), `transfer.rs` (döngünün pano tarafı). Gelenler `İndirilenler/OpenContinuity/<tarih>`; son 10 aktarım kalır.
- Arayüz: tür simgesi + yön rozeti, "gönderiliyor/alınıyor" satırı ve çizgide akış, sınır aşımı satırı, gelen dosyada "Göster", Ayarlar → Gelen dosyalar.
- Test planı: G0–G2, D0, D2 kayıtla işaretli; kalan G/D'yi kullanıcı toplu "tamam" dedi (madde madde işaretlenmedi).

Bilinen davranışlar (hata değil): Windows Ekran Alıntısı Aracı panoyu 0,3 sn sonra aynı içerikle yeniden yazar → aynı görsel 2 kez gider (kullanıcı filtre istemedi). Win+Shift+S alıntısı kopyalamadan panoya girer, Mac ⌘⇧4 dosyaya kaydeder. Nemo/Finder/Gezgin panodaki görsel verisini dosyaya çevirmez. KeePassXC süresi dolunca panoyu (başkasının içeriği olsa da) temizler. Kaldırılan cihaz o cihazla doğrudan yeniden eşleşmedikçe geri gelmez (arayüzde görünmüyor — öneri: Kaldırılanlar listesi).

0.1.5–0.1.7 (2026-10-18 akşam):
- Hareket azaltma (Windows "Animasyon efektleri" kapalı → `prefers-reduced-motion`): durum animasyonları kalır, yer değiştiren hareketler solmaya döner (`components.css` sonu). Gerçek cihazda onaylı.
- Kopukken yapılan son kopya 2 dk içinde bağlanılırsa kopyalandığı andaki Lamport'la gider (`transfer::Pending`, `Timing::offline_copy_max_age`, duvar saatiyle). Testli; **gerçek cihazda denenmedi**.
- Windows girişte başlatma: kayıt HKCU'ya, öneksiz/tırnaklı (`auto-launch` Dynamic kipi HKLM'ye `\\?\` önekli yazıyordu); her açılışta yenilenir, yalnız Ayarlar'dan kapatma `autostart-disabled` ile hatırlanır. Gerçek cihazda onaylı (F10).
- 0.1.7 noterlikli; Gatekeeper kabul ediyor.

Çoklu cihaz (2026-10-18 gece, `multi-device` dalı, yayınlanmadı → 0.1.8):
- Protokol **v6** (`Peers` mesajı): yeni cihaz gruptaki herhangi biriyle kod onaylar, diğerleri bağlantıdaki `Peers` alışverişiyle öğrenir. Tam ağ, aktarma yok. 0.1.7 ile bağlanmaz.
- `peers.json` (liste + `removed`); `peer.json` ilk yüklemede taşınır. Kaldırma yerel ve kalıcı (tanıtılsa da geri gelmez).
- Core: pano sürümü `(lamport, gönderen)` — eski "yalnız yerel kopyayla karşılaştır" kuralı 3 cihazda ayrışıyordu (test eski kodda kırmızı). `pending_echo` 4'lük kuyruk.
- Motor: `group.rs` (eş başına bağlantı/yeniden deneme/bekleyen kopya), `dial.rs` (çevirme ayrı görevde), `link.rs` ortak etiketli kanallar. Eşleştirme (2026-10-19'dan beri) senkronun yanında ayrı görev (`pairing.rs` + `pairing_host.rs`); gruptaki bağlantılar kopmaz. Eşleştirme açıkken dinleyici `Trust::PinnedOrPairing`: önce eşin `Hello`'sunu okur, gruptakini senkrona, yeni cihazı eşleştirmeye alır.
- Arayüz: grupta cihaz etiketleri, "Son aktarımlar"da kaynak adı, Ayarlar › Cihazlar (kart başına kaldır) + Cihaz ekle. Chrome önizlemesiyle doğrulandı (sahte Tauri: scratchpad `preview1/mock.js`).
- Gerçek cihaz testi yok: `docs/test-plan.md` M1–M6.

Linux (2026-10-19, `multi-device` dalı): Linux Mint / Cinnamon X11 hedef. `crates/clipboard/src/x11/` (x11rb; XFixes sayaç, sahip iş parçacığı + INCR, `image/png` baytları aynen, dosyalar `x-special/gnome-copied-files` + `text/uri-list`), `uri.rs` (testli). Wayland oturumunda arayüz uyarı kartı. Paket: `deb` + `AppImage` (`ubuntu-22.04`), güncelleyici ikisini de destekler (deb pkexec ile parola sorar), `latest.json`'da `linux-x86_64(-deb|-appimage)`. CI'a ubuntu + `xvfb-run` eklendi, `oc-linux` artifact'ı. Dal push'landı; CI 3 platformda yeşil (son 3 koşu, 9/9 iş); X11 pano testleri Xvfb'de gerçekten koşuyor ve geçiyor. Linux masaüstü paketi (`release.yml`) henüz hiç derlenmedi; gerçek Linux cihazda denenmedi.

Yedek arama (2026-10-19, `multi-device`): bağlantıyı asıl yönde küçük parmak izli taraf açar; öteki taraf `Timing::fallback_delay` (5 sn) içinde bağlantı gelmezse kendisi arar (senkron + eşleştirme). İki bağlantı birden kurulursa iki uç da asıl yöndekini tutar (`Member::replaces_link`); yarışta bir uç bir kez "koptu" görebilir, döngü yok. Protokol değişmedi. Neden: DentOs'ta (Mint 22.3, ufw açık, gelen TCP red) Mac Linux'a ulaşamıyordu; ufw mDNS'e varsayılan izin veriyor (`before.rules`). Gerçek ağda (Mac ↔ DentOs, Xvfb :99) doğrulandı: kendiliğinden eşleştirme, metin/görsel/klasör iki yönde hash eşit, yeniden başlatınca bağlanma, yankı yok. İkisi de gelen bağlantıyı engelliyorsa bağlanılamaz (çözümsüz). `oc clip-set*` Linux'ta başka kopya gelene kadar panoyu tutar (X11'de yazan süreç çıkınca pano boşalıyordu). Açık: Mac'te `oc clip-set-files` 2+ öğe yazınca ayrı süreç bazen 1 öğe görüyor (CLI çıkışı; uygulamada görülmedi), incelenmedi.

Kararsız testler (2026-10-19, çözüldü): yük altında bir uç el sıkışmayı zaman aşımıyla bırakırken öteki tamamlayıp kısa süre "bağlı" görünüyor (üretimde EOF ile kendini düzeltir). Testler artık `Node::wait_current` (son durum, kuyruktaki eski durum değil) ile iki ucu bekler, `Node::stop` motorun bitmesini bekler. Yerel yük testi: aynı anda 12 süreç → 2/240; izlenen tek hata `free_port` çakışması (test altyapısı).

iOS TestFlight (2026-10-19): ASC kaydı `OpenContinuity` (app id 6824589766), dahili grup "Dahili" (behcetozcann@gmail.com; egebusse@ eklenemedi — hesap uygulamayı görmüyor). Yükleme: imzasız archive + `xcodebuild -exportArchive` (destination upload, ASC API anahtarı); her yüklemede `CURRENT_PROJECT_VERSION` artır (son: 5, 0.1.11 simgesi). iPhone panosu GÖNDERİLMEZ (kullanıcı kararı): Mac'e Evrensel Pano taşıyor, ikisi birlikte döngü yaptı; gelen içerik panoya `localOnly` yazılır. Gerçek iPhone'da eşleşme, Mac → iPhone metin (Türkçe + emoji) ve görsel doğrulandı. Ana ekran: cihaz kartları + Gönder (Fotoğraf/Dosya kutuları).

iOS Faz 1 (2026-10-19, `ios` dalı, plan: araştırma `docs/ios-research.md`): uygulama açıkken iki yönlü metin/PNG/dosya senkronu, 6 haneli eşleştirme, Cihazlar/kaldır, Dosya gönder, gelen dosyalar Belgeler'de (Dosyalar uygulamasında görünür). Simülatörde doğrulandı (test planı "iOS" bölümü); gerçek iPhone'da denenmedi (I1–I8). Simülatör pano eşitlemesi kapatıldı (`defaults write com.apple.iphonesimulator PasteboardAutomaticSync -bool false`) — açıkken test içeriği Mac'teki gerçek uygulamaya, oradan gruba gider. DEBUG başlatma argümanları: `-ocAutoPair YES`, `-ocPushText <metin>`.

Sıradaki:
1. iOS: kalan I testleri (Windows/Linux → iPhone, klasör, Dosya gönder, Fotoğraf gönder, arka plan/yeniden bağlanma). Kilitlenince bağlantı kopması normal (iOS askıya alır); önerildi, onay bekliyor: süren aktarım için `beginBackgroundTask` (~30 sn). Ayarlar ekranı eski liste görünümünde; cihaz adı ağda "iPhone" (iOS gerçek adı vermiyor) → ad değiştirme ayarı önerisi. Sonra Faz 2: Paylaş uzantısı, App Intents (Gönder / Sonuncuyu Al), protokol v7 `FetchLatest` (1.0'dan önce girmeli). Faz 3: QR eşleştirme, `BGContinuedProcessingTask`, AB DMA talebi.
2. OpenContinuity için açık öneriler: Kaldırılanlar listesi + "X sizi kaldırdı" bildirimi; Windows kod imzası; gelen görseli dosya olarak kaydetme; saklanacak aktarım sayısı ayarı; M1–M6 (iOS uygulamasından sonra).
3. Açık bulgu: Windows Mac uykusunu ~4 dk geç fark etti (yeniden üretilmedi).

Açık kalanlar: gerçek simge tasarımı (yer tutucu); GitHub'da push/etiket iş akışını tetiklemiyor (elle `gh workflow run`), Actions token'ı Release oluşturamıyor (bu yüzden `scripts/publish-release.sh`); bu ağda GitHub API sık `connection reset` veriyor — betik tekrar çalıştırılabilir, `latest.json` CDN'de birkaç dakika eski kalabilir, doğrulamayı ondan yap.

## Yayın akışı

1. `Cargo.toml` `[workspace.package] version` artır, commit, `git tag -a vX.Y.Z`, push.
2. `gh workflow run release.yml -R behcetozcann/opencontinuity --ref vX.Y.Z` (etiket push'u şu an tetiklemiyor). Noterlik varsayılan açık (0.1.7'den beri; Apple 2026-10-18 akşamı onayladı). Apple kuyruğu takılırsa işi iptal edip `-f notarize=false` ile yeniden başlat (0.1.5'te iş saatlerce bekledi).
3. Derleme bitince `scripts/publish-release.sh vX.Y.Z --publish`: çıktıları indirir, `latest.json` üretir, Release'e yükler. Kurulu uygulamalar 6 saatte bir (ya da Ayarlar → Denetle) bulur.
4. Noterlik durumu: `scripts/notary-status.sh` (Apple'da web sayfası yok).

İmza/anahtarlar (repoda değil): güncelleyici anahtarı `~/.tauri/opencontinuity.key(.password)`; Developer ID `~/.tauri/developer-id.{key.pem,cer,p12,p12.password}` (G2, 2027-11-16'ya kadar); ASC API `~/.appstoreconnect/opencontinuity.env` + `AuthKey_3VZKZ58F7D.p8`. Hepsi GitHub Secrets'ta da var. Yerel derleme imzasızdır; dağıtım yalnız CI'dan.

## Gerçek cihaz test akışı

- Windows tarafını kullanıcı yürütür: `target/x86_64-pc-windows-gnu/release/oc.exe`'yi Masaüstüne kopyalar, PowerShell'de çalıştırır, çıktıyı yapıştırır.
- Mac tarafı arka planda: `./target/release/oc run > <scratchpad>/mac-run.log 2>&1 &`. Etkileşimli `oc pair` için `tail -f cevap.txt | oc pair`, kod eşleşince `echo e >> cevap.txt`.
- Protokol mesajı eklenirse `protocol::VERSION` artırılır; iki taraf aynı sürüme güncellenmeden bağlanmaz.
- Masaüstü uygulaması teşhis kaydını `<config>/oc.log`'a yazar (önceki oturum `oc.old.log`); config dizini macOS'ta `~/Library/Application Support/org.OpenContinuity.OpenContinuity`, Windows'ta `%APPDATA%\OpenContinuity\OpenContinuity\config`. Aynı Mac'te ikinci örnek: `OC_CONFIG_DIR=<dizin> OC_PORT=<port>` (tek örnek kilidi ve girişte başlatma o zaman devre dışı).
- Test için panoya yazma: `oc clip-set <metin>`, `oc clip-set-image <png>`, `oc clip-set-files <yollar…>`; okuma `oc clip-watch`. Gelen dosya klasörü `OC_FILES_DIR` ile değiştirilebilir.
- Teşhis için `oc run` her pano değişikliğinde sayaç, sahip süreç/biçimler ve hash öneki yazar; içerik yazılmaz. Tekrar/çift olayda önce bu kayıtla kaynağı ölç, filtre ekleme.
