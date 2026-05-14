#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NGINX="$REPO_ROOT/gateway/bin/nginx"
NGINX_PREFIX="$REPO_ROOT/gateway/nginx"
GW_ADMIN="$REPO_ROOT/target/debug/gw-admin"
BASE="http://localhost:9000"
PASS=0; FAIL=0

cleanup() {
    "$GW_ADMIN" stop \
        --nginx-prefix "$NGINX_PREFIX" \
        --nginx-bin "$NGINX" 2>/dev/null || true
    pkill -f "gw-admin" 2>/dev/null || true
    pkill -f "mock-upstream.py" 2>/dev/null || true
}
trap cleanup EXIT

check() {
    local desc="$1" expected="$2" actual="$3"
    if [ "$actual" = "$expected" ]; then
        echo "  PASS  $desc"
        PASS=$((PASS+1))
    else
        echo "  FAIL  $desc"
        echo "        expected: [$expected]"
        echo "        actual:   [$actual]"
        FAIL=$((FAIL+1))
    fi
}

# mock upstream + gw-admin start (background — stays alive for watcher)
python3 "$REPO_ROOT/gateway/scripts/mock-upstream.py" &
"$GW_ADMIN" start \
    --config-dir "$REPO_ROOT/gateway/config" \
    --nginx-prefix "$NGINX_PREFIX" \
    --nginx-bin "$NGINX" &
sleep 1

echo "mock upstream ready: svc_v1=127.0.0.1:8081, svc_v2=127.0.0.1:8082"

echo ""
echo "=== 1. 라우팅 ==="
check "GET /api/v1/ → svc_v1" "svc_v1" \
    "$(curl -s -H 'X-API-Key: secret-key-1' $BASE/api/v1/)"
check "GET /api/v2/ → svc_v2" "svc_v2" \
    "$(curl -s -H 'X-API-Key: secret-key-1' $BASE/api/v2/)"

echo ""
echo "=== 2. API Key 인증 ==="
check "유효한 Key → 200" "200" \
    "$(curl -s -o /dev/null -w '%{http_code}' -H 'X-API-Key: secret-key-1' $BASE/api/v1/)"
check "유효한 Key (2번) → 200" "200" \
    "$(curl -s -o /dev/null -w '%{http_code}' -H 'X-API-Key: secret-key-2' $BASE/api/v1/)"
check "잘못된 Key → 401" "401" \
    "$(curl -s -o /dev/null -w '%{http_code}' -H 'X-API-Key: wrong-key' $BASE/api/v1/)"
check "Key 없음 → 401" "401" \
    "$(curl -s -o /dev/null -w '%{http_code}' $BASE/api/v1/)"

echo ""
echo "=== 3. 헤더 조작 ==="
HEADERS=$(curl -s -H 'X-API-Key: secret-key-1' -D - -o /dev/null $BASE/api/v1/)
check "X-Gateway 응답 헤더 추가됨" "nginx-wasm-gw" \
    "$(echo "$HEADERS" | grep -i '^x-gateway:' | awk '{print $2}' | tr -d '\r')"
check "Server 헤더 제거됨" "" \
    "$(echo "$HEADERS" | grep -i '^server:' | awk '{print $2}' | tr -d '\r')"
check "X-Response-Time 응답 헤더 추가됨" "measured" \
    "$(echo "$HEADERS" | grep -i '^x-response-time:' | awk '{print $2}' | tr -d '\r')"

echo ""
echo "=== 4. Rate Limiting ==="
STATUSES=$(for i in $(seq 1 15); do
    curl -s -o /dev/null -w '%{http_code}\n' -H 'X-API-Key: rl-test-key' $BASE/api/v1/
done)
GOT_429=$(echo "$STATUSES" | grep -c "429" || true)
GOT_200=$(echo "$STATUSES" | grep -c "200" || true)
check "초기 요청들은 200" "true" "$([ "$GOT_200" -gt 0 ] && echo true || echo false)"
check "임계 초과 후 429 발생" "true" "$([ "$GOT_429" -gt 0 ] && echo true || echo false)"
echo "        (200: ${GOT_200}회, 429: ${GOT_429}회 / 총 15회)"

echo ""
echo "=== 5. JSON 로깅 ==="
curl -s -o /dev/null -H 'X-API-Key: secret-key-1' $BASE/api/v1/hello
sleep 0.3
LOG_LINE=$(grep '"method"' "$NGINX_PREFIX/logs/error.log" 2>/dev/null | tail -1)
check "JSON 로그 존재" "yes" "$([ -n "$LOG_LINE" ] && echo yes || echo no)"
[ -n "$LOG_LINE" ] && echo "        샘플: $(echo "$LOG_LINE" | grep -o '{.*}')"

echo ""
echo "================================"
echo "결과: ${PASS} passed, ${FAIL} failed"
echo "================================"

[ "$FAIL" -eq 0 ]
