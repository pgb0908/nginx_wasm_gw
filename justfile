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

# wasm 필터 빌드
build:
    {{cargo}} build --workspace --target wasm32-wasip1 --profile wasm-release

# mock upstream 시작 (백그라운드, 8081/8082)
mock:
    @echo "mock upstream 시작 (svc_v1:8081, svc_v2:8082)"
    @python3 gateway/scripts/mock-upstream.py &
    @sleep 0.5 && echo "mock upstream ready"

# nginx 시작
nginx:
    cd {{nginx_prefix}} && ../bin/nginx -p . -c nginx.conf

# nginx 중지
stop:
    -cd {{nginx_prefix}} && ../bin/nginx -p . -s stop 2>/dev/null
    -pkill -f "mock-upstream.py" 2>/dev/null
    @echo "stopped"

# nginx 설정 검증
check:
    cd {{nginx_prefix}} && ../bin/nginx -p . -c nginx.conf -t

# 통합 테스트 (mock + nginx 자동 관리)
integration: build
    @bash gateway/scripts/integration-test.sh
