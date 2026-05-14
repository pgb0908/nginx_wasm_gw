set dotenv-load := false
set shell := ["bash", "-c"]

cargo := "$HOME/.cargo/bin/cargo"
nginx := "gateway/bin/nginx"
nginx_prefix := "gateway/nginx"

# 사용 가능한 명령어 목록
default:
    @just --list

# 유닛 테스트
test:
    {{cargo}} test --workspace

# wasm 필터 빌드 (gateway-admin은 native 바이너리이므로 제외)
build:
    {{cargo}} build --workspace --target wasm32-wasip1 --profile wasm-release --exclude gateway-admin

# mock upstream 시작 (백그라운드, 8081/8082)
mock:
    @echo "mock upstream 시작 (svc_v1:8081, svc_v2:8082)"
    @python3 gateway/scripts/mock-upstream.py &
    @sleep 0.5 && echo "mock upstream ready"

# gw-admin 빌드
build-admin:
    {{cargo}} build -p gateway-admin

# nginx 시작 (gw-admin 래퍼)
nginx: build-admin
    ./target/debug/gw-admin start

# nginx 중지 (gw-admin 래퍼)
stop:
    -./target/debug/gw-admin stop 2>/dev/null
    -pkill -f "gw-admin" 2>/dev/null
    -pkill -f "mock-upstream.py" 2>/dev/null
    @echo "stopped"

# nginx 설정 검증
check:
    cd {{nginx_prefix}} && ../bin/nginx -p . -c nginx.conf -t

# 통합 테스트 (mock + nginx 자동 관리)
integration: build build-admin
    @bash gateway/scripts/integration-test.sh

# 릴리즈 번들 생성 (ADR-0004)
release:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(git describe --tags --exact-match 2>/dev/null || echo "v0.0.0-dev")
    BUNDLE="nginx-wasm-gw-${VERSION}-linux-x86_64"
    echo "Building release bundle: ${BUNDLE}.tar.gz"

    $HOME/.cargo/bin/cargo build --workspace --target wasm32-wasip1 --profile wasm-release --exclude gateway-admin
    $HOME/.cargo/bin/cargo build -p gateway-admin --release --target x86_64-unknown-linux-musl

    rm -rf "${BUNDLE}"
    mkdir -p "${BUNDLE}/bin" \
             "${BUNDLE}/filters" \
             "${BUNDLE}/nginx/logs" \
             "${BUNDLE}/nginx/tmp/client_body" \
             "${BUNDLE}/nginx/tmp/proxy" \
             "${BUNDLE}/nginx/tmp/fastcgi" \
             "${BUNDLE}/nginx/tmp/scgi" \
             "${BUNDLE}/nginx/tmp/uwsgi"

    cp target/x86_64-unknown-linux-musl/release/gw-admin "${BUNDLE}/bin/"
    cp gateway/bin/nginx "${BUNDLE}/bin/"
    for f in api_key rate_limit header_manipulation logging passthrough; do
        cp "target/wasm32-wasip1/wasm-release/${f}.wasm" "${BUNDLE}/filters/"
    done
    cp -r gateway/config "${BUNDLE}/config"
    cp -r gateway/config-loadtest "${BUNDLE}/config-loadtest"
    cp README.md "${BUNDLE}/"

    tar -czf "${BUNDLE}.tar.gz" "${BUNDLE}"
    rm -rf "${BUNDLE}"
    echo "Created: ${BUNDLE}.tar.gz"
