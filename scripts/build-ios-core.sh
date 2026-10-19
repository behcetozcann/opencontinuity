#!/usr/bin/env bash
# Rust çekirdeğini iOS (cihaz + Apple Silicon simülatör) için derler, UniFFI
# Swift bağlarını üretir ve apps/ios/Core altına xcframework olarak koyar.
# Kullanım: scripts/build-ios-core.sh [debug]   (varsayılan release)
set -euo pipefail

cd "$(dirname "$0")/.."
source "$HOME/.cargo/env"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

PROFILE="${1:-release}"
FLAG=$([ "$PROFILE" = release ] && echo --release || true)
LIB=libopencontinuity_core.a
OUT=apps/ios/Core
GEN="$(mktemp -d)"
trap 'rm -rf "$GEN"' EXIT

for target in aarch64-apple-ios aarch64-apple-ios-sim; do
    cargo build -p oc-ffi --lib $FLAG --target "$target"
done

# Bağlar kütüphanenin içindeki meta veriden üretilir (Mac'te derlenmiş dylib).
cargo build -p oc-ffi --lib $FLAG
cargo run -q -p oc-ffi --bin uniffi-bindgen -- generate \
    --library "target/$PROFILE/libopencontinuity_core.dylib" --language swift --out-dir "$GEN"

rm -rf "$OUT" && mkdir -p "$OUT/Headers"
cp "$GEN"/*.h "$OUT/Headers/"
cat "$GEN"/*.modulemap > "$OUT/Headers/module.modulemap"
cp "$GEN"/*.swift "$OUT/"

xcodebuild -create-xcframework \
    -library "target/aarch64-apple-ios/$PROFILE/$LIB" -headers "$OUT/Headers" \
    -library "target/aarch64-apple-ios-sim/$PROFILE/$LIB" -headers "$OUT/Headers" \
    -output "$OUT/OpenContinuityCore.xcframework" >/dev/null
rm -rf "$OUT/Headers"
echo "Hazır: $OUT"
