# OpenContinuity — proje notları

Mac ↔ Windows yerel ağ pano senkronizasyonu (Rust). Ürün kapsamı ve kararlar: `OpenContinuity_Proje_Plani.md`. Belgeler `docs/` altında.

## Komutlar

```sh
source "$HOME/.cargo/env"
cargo build                                        # macOS
cargo build --target x86_64-pc-windows-gnu         # Windows .exe (mingw-w64, .cargo/config.toml)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd apps/desktop/src-tauri && cargo tauri build --bundles app,dmg                          # macOS paketi (yerel, imzasız; yayın CI'dan)
# Windows installer: msvc + cargo-xwin (gnu hedefi WebView2Loader.dll'e bağımlı kalıyor, kullanıcıda 'bulunamadı' hatası verdi)
cd apps/desktop/src-tauri && PATH="/opt/homebrew/opt/llvm/bin:$PATH" cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
```

## Yapı

- `crates/engine` → senkron motoru (eşleştirme + senkron döngüsü, olay/komut API'si); CLI ve masaüstü bunu kullanır
- `apps/cli` → `oc` binary'si (teşhis aracı)
- `apps/desktop` → Tauri 2 uygulaması (`src-tauri/` Rust, `ui/` derleme adımsız HTML/JS)
- `crates/<ad>` → kütüphaneler; her yeni crate `Cargo.toml` `members` listesine eklenir (glob boş eşleşirse cargo hata verir)

## Kurallar

- Pano içeriği hiçbir log, test çıktısı veya hata mesajına yazılmaz; yalnız uzunluk + hash öneki.
- Kriptografi kendimiz yazılmaz (rustls/ring, rcgen, sha2).
- Gerçek cihaz testi yapılmadıysa `docs/test-plan.md`'de işaretlenmez.
- Dosya başına 300 satırı geçme.

## Durum (2026-10-18)

Faz 1 kodu `main`'de; depo https://github.com/behcetozcann/opencontinuity (herkese açık, PolyForm Noncommercial). Yayında: v0.1.1. v0.1.2 (yeni arayüz + Developer ID imzası + noterlik) derleniyor; ilk noterlik Apple'da bekliyor.

Sıradaki: v0.1.2 yayını → Mac'te Yerel Ağ iznini bir kez ver (imza artık sabit) → Windows'u güncelle → sihirbazla eşleştir → `docs/test-plan.md` Faz 1 tablosu (F1–F12).

Açık kalanlar: gerçek simge tasarımı (yer tutucu); GitHub'da push/etiket iş akışını tetiklemiyor (elle `gh workflow run`), Actions token'ı Release oluşturamıyor (bu yüzden `scripts/publish-release.sh`); `icons/` altında `cargo tauri icon`'un ürettiği izlenmeyen mobil simgeler (silmek için kullanıcı onayı gerekli).

## Yayın akışı

1. `Cargo.toml` `[workspace.package] version` artır, commit, `git tag -a vX.Y.Z`, push.
2. `gh workflow run release.yml -R behcetozcann/opencontinuity --ref vX.Y.Z` (etiket push'u şu an tetiklemiyor).
3. Derleme bitince `scripts/publish-release.sh vX.Y.Z --publish`: çıktıları indirir, `latest.json` üretir, Release'e yükler. Kurulu uygulamalar 6 saatte bir (ya da Ayarlar → Denetle) bulur.
4. Noterlik durumu: `scripts/notary-status.sh` (Apple'da web sayfası yok).

İmza/anahtarlar (repoda değil): güncelleyici anahtarı `~/.tauri/opencontinuity.key(.password)`; Developer ID `~/.tauri/developer-id.{key.pem,cer,p12,p12.password}` (G2, 2027-11-16'ya kadar); ASC API `~/.appstoreconnect/opencontinuity.env` + `AuthKey_3VZKZ58F7D.p8`. Hepsi GitHub Secrets'ta da var. Yerel derleme imzasızdır; dağıtım yalnız CI'dan.

## Gerçek cihaz test akışı

- Windows tarafını kullanıcı yürütür: `target/x86_64-pc-windows-gnu/release/oc.exe`'yi Masaüstüne kopyalar, PowerShell'de çalıştırır, çıktıyı yapıştırır.
- Mac tarafı arka planda: `./target/release/oc run > <scratchpad>/mac-run.log 2>&1 &`. Etkileşimli `oc pair` için `tail -f cevap.txt | oc pair`, kod eşleşince `echo e >> cevap.txt`.
- Protokol mesajı eklenirse `protocol::VERSION` artırılır; iki taraf aynı sürüme güncellenmeden bağlanmaz.
- Masaüstü uygulaması teşhis kaydını `<config>/oc.log`'a yazar (önceki oturum `oc.old.log`); config dizini macOS'ta `~/Library/Application Support/org.OpenContinuity.OpenContinuity`, Windows'ta `%APPDATA%\OpenContinuity\OpenContinuity\config`. Aynı Mac'te ikinci örnek: `OC_CONFIG_DIR=<dizin> OC_PORT=<port>` (tek örnek kilidi ve girişte başlatma o zaman devre dışı).
- Teşhis için `oc run` her pano değişikliğinde sayaç, sahip süreç/biçimler ve hash öneki yazar; içerik yazılmaz. Tekrar/çift olayda önce bu kayıtla kaynağı ölç, filtre ekleme.
