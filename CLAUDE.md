# OpenContinuity — proje notları

Mac ↔ Windows yerel ağ pano senkronizasyonu (Rust). Ürün kapsamı ve kararlar: `OpenContinuity_Proje_Plani.md`. Belgeler `docs/` altında.

## Komutlar

```sh
source "$HOME/.cargo/env"
cargo build                                        # macOS
cargo build --target x86_64-pc-windows-gnu         # Windows .exe (mingw-w64, .cargo/config.toml)
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Yapı

- `apps/cli` → `oc` binary'si
- `crates/<ad>` → kütüphaneler; her yeni crate `Cargo.toml` `members` listesine eklenir (glob boş eşleşirse cargo hata verir)

## Kurallar

- Pano içeriği hiçbir log, test çıktısı veya hata mesajına yazılmaz; yalnız uzunluk + hash öneki.
- Kriptografi kendimiz yazılmaz (rustls/ring, rcgen, sha2).
- Gerçek cihaz testi yapılmadıysa `docs/test-plan.md`'de işaretlenmez.
- Dosya başına 300 satırı geçme.

## Durum (2026-10-17)

Faz 0 PoC tamam: `oc pair` (mDNS ile IP'siz, 6 haneli SAS), `oc run` (iki yönlü düz metin, heartbeat, yeniden bağlanma, Lamport saat eşitleme), protokol v2. Ayrıntılı sonuçlar ve bulgular: `docs/test-plan.md` → "Bulgular".

Açık kalanlar: Mac→Windows emoji/çok satır görsel kontrolü; hızlı yeniden bağlanma düzeltmesinin cihazda yeniden üretilmesi; heartbeat ve `unpair` sonrası red için otomatik test; macOS pano gizlilik uyarısının uygulama paketinde ölçülmesi; tekrar gönderimin sync programı açıkken sahip adıyla doğrulanması.

Sıradaki: **Faz 1** — Tauri 2 menü çubuğu / sistem tepsisi kabuğu, açılışta otomatik başlatma, ilk eşleştirme arayüzü, Windows installer + macOS dağıtımı (plan: `OpenContinuity_Proje_Plani.md` §9).

## Gerçek cihaz test akışı

- Windows tarafını kullanıcı yürütür: `target/x86_64-pc-windows-gnu/release/oc.exe`'yi Masaüstüne kopyalar, PowerShell'de çalıştırır, çıktıyı yapıştırır.
- Mac tarafı arka planda: `./target/release/oc run > <scratchpad>/mac-run.log 2>&1 &`. Etkileşimli `oc pair` için `tail -f cevap.txt | oc pair`, kod eşleşince `echo e >> cevap.txt`.
- Protokol mesajı eklenirse `protocol::VERSION` artırılır; iki taraf aynı sürüme güncellenmeden bağlanmaz.
- Teşhis için `oc run` her pano değişikliğinde sayaç, sahip süreç/biçimler ve hash öneki yazar; içerik yazılmaz. Tekrar/çift olayda önce bu kayıtla kaynağı ölç, filtre ekleme.
