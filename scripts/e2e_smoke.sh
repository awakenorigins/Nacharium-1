#!/usr/bin/env bash
# S6.3 — End-to-end smoke test against a running nacharium-node.
# Boots the release binary against live Postgres, then exercises the full
# citizen lifecycle: veil -> register(+13 tokens) -> wallet -> agent intent.
set -euo pipefail

BIN="${BIN:-./target/release/nacharium-node}"
export DATABASE_URL="${DATABASE_URL:-postgres://nacha:nacha@127.0.0.1:5432/nacharium}"
BASE="http://127.0.0.1:8787/api/v1"

echo "== booting nacharium-node =="
"$BIN" &
SRV=$!
trap 'kill $SRV 2>/dev/null || true' EXIT

# wait for the port
for i in $(seq 1 30); do
  if curl -sf "$BASE/proto/system/laws" >/dev/null 2>&1; then break; fi
  sleep 0.5
done

pass=0; fail=0
check() { # name, jq-expr, actual-json
  if echo "$3" | jq -e "$2" >/dev/null 2>&1; then
    echo "  PASS: $1"; pass=$((pass+1))
  else
    echo "  FAIL: $1 -> $3"; fail=$((fail+1))
  fi
}

echo "== 1. system laws =="
LAWS=$(curl -sf "$BASE/proto/system/laws")
check "7 protocols present" '.protocols | length == 7' "$LAWS"
check "7 physics laws present" '.physics | length == 7' "$LAWS"

echo "== 2. enter as Veil (Genesis Seven) =="
VEIL=$(curl -sf -X POST "$BASE/veil")
DID=$(echo "$VEIL" | jq -r .veil_did)
check "veil did issued" '.veil_did | startswith("did:nch:")' "$VEIL"
check "7 prototypes granted" '.prototypes | length == 7' "$VEIL"

echo "== 3. register -> 13 tokens =="
REG=$(curl -sf -X POST "$BASE/veil/register" -H 'content-type: application/json' \
  -d "{\"veil_did\":\"$DID\",\"nick\":\"smoke-$(date +%s%N)\"}")
check "granted 13 tokens" '.tokens_granted == 13' "$REG"

echo "== 4. wallet reflects 13 NARI =="
WAL=$(curl -sf "$BASE/wallet/$DID")
check "wallet = 13 NARI" '.balance_nari == 13' "$WAL"

echo "== 5. agent intent compatibility =="
INT=$(curl -sf -X POST "$BASE/agent/intent" -H 'content-type: application/json' \
  -d '{"actor_did":"did:nch:a","want":"x","why":"y","needs_perms":["read"],"risk":5,"deadline":9999999999,"granted_perms":["read"],"now":1}')
check "intent compatible" '.compatible == true' "$INT"

echo "== 6. sanctum seals plaintext (server sees ciphertext only) =="
SANC=$(curl -sf -X POST "$BASE/sanctum/demo/message" -H 'content-type: application/json' \
  -d '{"veil_did":"did:nch:a","text":"classified"}')
check "no plaintext leak" '.plaintext_leaked == false' "$SANC"

echo "== RESULT: $pass passed / $fail failed =="
[ "$fail" -eq 0 ]
