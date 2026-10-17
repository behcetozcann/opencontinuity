"""App Store Connect API için ES256 JWT (openssl ile imzalar; ek kütüphane gerekmez)."""
import base64, json, os, subprocess, sys, time

def b64(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

env = dict(line.strip().split("=", 1) for line in open(os.path.expanduser("~/.appstoreconnect/opencontinuity.env")) if "=" in line)
header = {"alg": "ES256", "kid": env["APPLE_API_KEY"], "typ": "JWT"}
now = int(time.time())
payload = {"iss": env["APPLE_API_ISSUER"], "iat": now, "exp": now + 600, "aud": "appstoreconnect-v1"}
signing_input = f"{b64(json.dumps(header).encode())}.{b64(json.dumps(payload).encode())}".encode()
der = subprocess.run(["openssl", "dgst", "-sha256", "-sign", env["APPLE_API_KEY_PATH"]], input=signing_input, capture_output=True, check=True).stdout
# DER ECDSA imzası → ham r||s (32+32 bayt)
assert der[0] == 0x30
idx = 2
parts = []
for _ in range(2):
    assert der[idx] == 0x02
    length = der[idx + 1]
    parts.append(der[idx + 2: idx + 2 + length].lstrip(b"\x00").rjust(32, b"\x00"))
    idx += 2 + length
sys.stdout.write(f"{signing_input.decode()}.{b64(parts[0] + parts[1])}")
