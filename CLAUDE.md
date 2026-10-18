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
```

## Yapı

- `crates/engine` → senkron motoru (eşleştirme + gruptaki eşlerle senkron döngüsü, olay/komut API'si); CLI ve masaüstü bunu kullanır
- `apps/cli` → `oc` binary'si (teşhis aracı)
- `apps/desktop` → Tauri 2 uygulaması (`src-tauri/` Rust, `ui/` derleme adımsız HTML/JS)
- `crates/<ad>` → kütüphaneler; her yeni crate `Cargo.toml` `members` listesine eklenir (glob boş eşleşirse cargo hata verir)

## Kurallar

- Pano içeriği hiçbir log, test çıktısı veya hata mesajına yazılmaz; yalnız uzunluk + hash öneki.
- Kriptografi kendimiz yazılmaz (rustls/ring, rcgen, sha2).
- Gerçek cihaz testi yapılmadıysa `docs/test-plan.md`'de işaretlenmez.
- Dosya başına 300 satırı geçme.

## Durum (2026-10-18, oturum sonu)

Depo https://github.com/behcetozcann/opencontinuity (herkese açık, PolyForm Noncommercial). Yayında: **v0.1.7** (protokol v5: metin + görsel + dosya/klasör). macOS paketi Developer ID ile imzalı (Team LKD3F8VA6H, kimlik `org.opencontinuity.desktop`) ve 0.1.7'den beri **noterlikli** (DMG içindeki uygulama `spctl`: accepted, Notarized Developer ID; staple doğrulandı).

Faz 2 yapılanlar (hepsi gerçek cihazda kullanıcı onaylı):
- Görsel panosu (0.1.3, v4): PNG, ≤ 32 MiB, 256 KiB parçalar. macOS `public.png`/TIFF→PNG; Windows `PNG` biçimi (IEND'e göre kırpılır) / CF_DIB→PNG, yazarken PNG + CF_DIB. Metin + görsel birlikteyse metin gider.
- Dosya/klasör (0.1.4, v5): ≤ 500 MiB, 5.000 girdi; `engine/src/files.rs` (toplama, diske yazma, kabul/budama), `writer.rs` (ayrı yazma görevi; ping araya girer, yeni kopya süren aktarımı `TransferCancel` ile bırakır), `transfer.rs` (döngünün pano tarafı). Gelenler `İndirilenler/OpenContinuity/<tarih>`; son 10 aktarım kalır.
- Arayüz: tür simgesi + yön rozeti, "gönderiliyor/alınıyor" satırı ve çizgide akış, sınır aşımı satırı, gelen dosyada "Göster", Ayarlar → Gelen dosyalar.
- Test planı: G0–G2, D0, D2 kayıtla işaretli; kalan G/D'yi kullanıcı toplu "tamam" dedi (madde madde işaretlenmedi).

Bilinen davranışlar (hata değil): Windows Ekran Alıntısı Aracı panoyu 0,3 sn sonra aynı içerikle yeniden yazar → aynı görsel 2 kez gider (kullanıcı filtre istemedi). Win+Shift+S alıntısı kopyalamadan panoya girer, Mac ⌘⇧4 dosyaya kaydeder.

0.1.5–0.1.7 (2026-10-18 akşam):
- Hareket azaltma (Windows "Animasyon efektleri" kapalı → `prefers-reduced-motion`): durum animasyonları kalır, yer değiştiren hareketler solmaya döner (`components.css` sonu). Gerçek cihazda onaylı.
- Kopukken yapılan son kopya 2 dk içinde bağlanılırsa kopyalandığı andaki Lamport'la gider (`transfer::Pending`, `Timing::offline_copy_max_age`, duvar saatiyle). Testli; **gerçek cihazda denenmedi**.
- Windows girişte başlatma: kayıt HKCU'ya, öneksiz/tırnaklı (`auto-launch` Dynamic kipi HKLM'ye `\\?\` önekli yazıyordu); her açılışta yenilenir, yalnız Ayarlar'dan kapatma `autostart-disabled` ile hatırlanır. Gerçek cihazda onaylı (F10).
- 0.1.7 noterlikli; Gatekeeper kabul ediyor.

Çoklu cihaz (2026-10-18 gece, `multi-device` dalı, yayınlanmadı → 0.1.8):
- Protokol **v6** (`Peers` mesajı): yeni cihaz gruptaki herhangi biriyle kod onaylar, diğerleri bağlantıdaki `Peers` alışverişiyle öğrenir. Tam ağ, aktarma yok. 0.1.7 ile bağlanmaz.
- `peers.json` (liste + `removed`); `peer.json` ilk yüklemede taşınır. Kaldırma yerel ve kalıcı (tanıtılsa da geri gelmez).
- Core: pano sürümü `(lamport, gönderen)` — eski "yalnız yerel kopyayla karşılaştır" kuralı 3 cihazda ayrışıyordu (test eski kodda kırmızı). `pending_echo` 4'lük kuyruk.
- Motor: `group.rs` (eş başına bağlantı/yeniden deneme/bekleyen kopya), `dial.rs` (çevirme ayrı görevde), `link.rs` ortak etiketli kanallar. Eşleştirme sırasında senkron durur.
- Arayüz: grupta cihaz etiketleri, "Son aktarımlar"da kaynak adı, Ayarlar › Cihazlar (kart başına kaldır) + Cihaz ekle. Chrome önizlemesiyle doğrulandı (sahte Tauri: scratchpad `preview1/mock.js`).
- Gerçek cihaz testi yok: `docs/test-plan.md` M1–M6.

Linux (2026-10-19, `multi-device` dalı): Linux Mint / Cinnamon X11 hedef. `crates/clipboard/src/x11/` (x11rb; XFixes sayaç, sahip iş parçacığı + INCR, `image/png` baytları aynen, dosyalar `x-special/gnome-copied-files` + `text/uri-list`), `uri.rs` (testli). Wayland oturumunda arayüz uyarı kartı. Paket: `deb` + `AppImage` (`ubuntu-22.04`), güncelleyici ikisini de destekler (deb pkexec ile parola sorar), `latest.json`'da `linux-x86_64(-deb|-appimage)`. CI'a ubuntu + `xvfb-run` eklendi, `oc-linux` artifact'ı. **CI hiç koşmadı** (dal push'lanmadı) — engine/masaüstü Linux derlemesi ve X11 testleri ilk CI koşusunda doğrulanacak.

Sıradaki:
1. Dalı push'la (kullanıcı onayı) → CI (Linux derleme + Xvfb testleri) → release.yml'i dalda elle tetikle → kullanıcı Linux'ta `.deb` kurup test-plan L1–L7.
2. Toplu gerçek cihaz testi: M1–M6 + kopukken kopya (Mac kapak kapat/aç, bağlanmadan kopyala → Windows'ta yapıştır). Kullanıcı testleri biriktirmek istedi.
3. Açık bulgu: Windows Mac uykusunu ~4 dk geç fark etti (yeniden üretilmedi).
4. Diğer Faz 2: pano geçmişi/tercihler, isteğe bağlı "dosya paylaşımı" anahtarı.

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
