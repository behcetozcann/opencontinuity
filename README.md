# OpenContinuity

İki cihazı aç, bir kez eşleştir; birinde kopyaladığını diğerinde normal yapıştır.

Açık kaynaklı, sunucusuz, yerel ağda şifreli çalışan cihazlar arası pano senkronizasyonu. İlk hedef: Windows ↔ macOS arasında iki yönlü düz metin.

> **Durum:** Faz 0 PoC — kullanıma hazır değil. Ürün planı: [`OpenContinuity_Proje_Plani.md`](OpenContinuity_Proje_Plani.md).

## Derleme

```sh
cargo build --release                                   # bu makine
cargo build --release --target x86_64-pc-windows-gnu    # Mac'ten Windows .exe (mingw-w64 gerekir)
cargo test --workspace
```

## Belgeler

- [PRD](docs/PRD.md) — ne yapıyoruz
- [Mimari](docs/architecture.md) — nasıl yapıyoruz
- [Protokol](docs/protocol.md)
- [Güvenlik modeli](docs/security-model.md)
- [Test planı](docs/test-plan.md)

## Lisans

[MIT](LICENSE)
