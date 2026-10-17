#!/usr/bin/env bash
# Release iş akışının çıktılarını indirir, güncelleyici için latest.json üretir
# ve GitHub Release'e yükler. Kullanım:
#   scripts/publish-release.sh v0.1.1            # taslak olarak yükle
#   scripts/publish-release.sh v0.1.1 --publish  # yükle ve yayınla
set -euo pipefail

REPO="behcetozcann/opencontinuity"
TAG="${1:?kullanım: $0 vX.Y.Z [--publish]}"
PUBLISH="${2:-}"
VERSION="${TAG#v}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

RUN_ID="$(gh run list -R "$REPO" --workflow release.yml --status success --limit 20 \
  --json databaseId,headBranch --jq "[.[] | select(.headBranch == \"$TAG\" or .headBranch == \"main\")][0].databaseId")"
[ -n "$RUN_ID" ] && [ "$RUN_ID" != "null" ] || { echo "başarılı Release derlemesi bulunamadı" >&2; exit 1; }
echo "Derleme: https://github.com/$REPO/actions/runs/$RUN_ID"
gh run download "$RUN_ID" -R "$REPO" -D "$WORK/raw"

ASSETS="$WORK/assets"
mkdir -p "$ASSETS"
find "$WORK/raw" -type f -exec cp {} "$ASSETS/" \;
# Güncelleme paketi sürüm adını taşımıyor; karışmasın diye adlandırılır.
mv "$ASSETS/OpenContinuity.app.tar.gz" "$ASSETS/OpenContinuity_${VERSION}_universal.app.tar.gz"
mv "$ASSETS/OpenContinuity.app.tar.gz.sig" "$ASSETS/OpenContinuity_${VERSION}_universal.app.tar.gz.sig"

for expected in "OpenContinuity_${VERSION}_universal.dmg" "OpenContinuity_${VERSION}_universal.app.tar.gz" \
  "OpenContinuity_${VERSION}_x64-setup.exe"; do
  [ -f "$ASSETS/$expected" ] || { echo "eksik dosya: $expected (sürüm uyuşmuyor mu?)" >&2; ls "$ASSETS" >&2; exit 1; }
done

python3 - "$ASSETS" "$VERSION" "$TAG" "$REPO" <<'EOF'
import json, sys, datetime, pathlib
assets, version, tag, repo = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
base = f"https://github.com/{repo}/releases/download/{tag}"
def entry(name):
    return {"signature": (assets / f"{name}.sig").read_text().strip(), "url": f"{base}/{name}"}
mac = entry(f"OpenContinuity_{version}_universal.app.tar.gz")
windows = entry(f"OpenContinuity_{version}_x64-setup.exe")
latest = {
    "version": version,
    "notes": f"OpenContinuity {tag}",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "platforms": {
        "darwin-aarch64": mac, "darwin-x86_64": mac,
        "windows-x86_64": windows, "windows-x86_64-nsis": windows,
    },
}
(assets / "latest.json").write_text(json.dumps(latest, indent=2) + "\n")
EOF

NOTES="**Windows:** \`OpenContinuity_${VERSION}_x64-setup.exe\` dosyasını indirip kurun. SmartScreen uyarısında \"Ek bilgi → Yine de çalıştır\".
**macOS:** \`OpenContinuity_${VERSION}_universal.dmg\` dosyasını indirin, uygulamayı Applications'a sürükleyin. İlk açılışta sağ tık → Aç.

Kurulu uygulamalar yeni sürümü kendiliğinden bulur."

if gh release view "$TAG" -R "$REPO" >/dev/null 2>&1; then
  gh release upload "$TAG" -R "$REPO" --clobber "$ASSETS"/*
  gh release edit "$TAG" -R "$REPO" --title "OpenContinuity $TAG" --notes "$NOTES"
else
  gh release create "$TAG" -R "$REPO" --draft --verify-tag --title "OpenContinuity $TAG" --notes "$NOTES" "$ASSETS"/*
fi

if [ "$PUBLISH" = "--publish" ]; then
  gh release edit "$TAG" -R "$REPO" --draft=false --latest
  echo "Yayınlandı: https://github.com/$REPO/releases/tag/$TAG"
else
  echo "Taslak hazır: https://github.com/$REPO/releases (yayınlamak için --publish)"
fi
