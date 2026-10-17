# Protokol v1 (taslak)

## Taşıma

Karşılıklı kimlik doğrulamalı TLS 1.3 üzerinde tek TCP bağlantısı. Sertifikalar cihaz başına self-signed; güven, eşleştirmede doğrulanan SHA-256 parmak izine dayanır.

## Çerçeve

```text
┌──────────────────────┬────────────────────────────┐
│ uzunluk: u32 (BE)    │ gövde: postcard(Message)   │
└──────────────────────┴────────────────────────────┘
```

- Gövde en fazla **5 MiB**; aşan çerçeve bağlantıyı kapatır.
- Metin en fazla **4 MiB** (UTF-8 bayt).

## Mesajlar

| Mesaj | Alanlar | Not |
|---|---|---|
| `Hello` | `version: u16`, `device_id: [u8; 16]`, `name: String` | Bağlantıdaki ilk mesaj; sürüm uyuşmazsa bağlantı kapanır |
| `Clip` | `event_id: [u8; 16]`, `lamport: u64`, `text: String` | Düz metin pano olayı |
| `Ping` | `nonce: u64` | 5 sn'de bir |
| `Pong` | `nonce: u64` | |

`device_id` sertifikanın parmak izinden türetilir (ilk 16 bayt), böylece taklit edilemez.

## Eşleştirme (SAS)

1. Taraflar, eşleşmemiş sertifikayı geçici olarak kabul eden bir TLS bağlantısı kurar.
2. Her iki uç `SHA-256("oc-sas-v1" ‖ min(fp_a, fp_b) ‖ max(fp_a, fp_b) ‖ tls_exporter("oc-sas", 32))` hesaplar; ilk 8 baytın `u64 % 1_000_000` değeri 6 haneli koddur.
3. Kullanıcı iki ekranda aynı kodu görüp iki uçta da onaylar; onaylanmazsa hiçbir şey kaydedilmez.
4. Onaydan sonra karşı tarafın parmak izi `peers.json`'a yazılır; sonraki bağlantılarda yalnız bu parmak izi kabul edilir.
