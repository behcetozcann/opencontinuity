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
