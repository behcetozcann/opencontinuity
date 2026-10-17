#!/usr/bin/env bash
# Apple noterlik gönderimlerinin son durumunu gösterir.
# Gerekli: ~/.appstoreconnect/opencontinuity.env (APPLE_API_ISSUER, APPLE_API_KEY, APPLE_API_KEY_PATH)
set -euo pipefail
TOKEN="$(python3 "$(dirname "$0")/asc_jwt.py")"
curl -s -H "Authorization: Bearer $TOKEN" "https://appstoreconnect.apple.com/notary/v2/submissions" |
  python3 -c "import json,sys; [print(s['attributes']['status'], s['attributes']['createdDate'], s['attributes']['name']) for s in json.load(sys.stdin).get('data', [])[:5]]"
