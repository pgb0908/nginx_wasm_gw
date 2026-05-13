# Context: nginx-wasm-gw

## What this is

ngx_wasm_module(Proxy-Wasm)을 기반으로 한 **API Gateway**. 외부 클라이언트의 요청을 받아 내부 업스트림 서비스로 라우팅하며, Wasm 필터 체인으로 공통 횡단 관심사를 처리한다.

## Glossary

### API Gateway
외부 클라이언트 → 내부 업스트림 서비스 사이의 단일 진입점. 인증, 라우팅, 변환, 관찰성 등의 횡단 관심사를 처리한다. Ingress Controller나 Service Mesh Sidecar와 구별된다.

### Wasm Filter
Rust로 작성되어 `.wasm`으로 컴파일된 플러그인 단위. ngx_wasm_module의 샌드박스 안에서 nginx 요청 처리 파이프라인에 삽입된다. 각 필터는 하나의 횡단 관심사를 담당한다.

### Filter Chain
요청/응답 처리 파이프라인에 순서대로 삽입된 Wasm Filter의 집합. 각 필터는 **우선순위(priority) 값**을 가지며, 낮은 값이 먼저 실행된다. 순서는 `nginx.conf`에 필터를 나열하는 순서로 결정된다. 권장 우선순위: Rate Limiting(1) → API Key 인증(2) → 헤더 조작(3) → 로깅(4).

### Upstream
API Gateway가 요청을 전달하는 내부 백엔드 서비스. `nginx.conf`의 `upstream` 블록으로 정의하며, 경로 기반 라우팅은 nginx의 `proxy_pass`가 담당한다. Wasm Filter는 라우팅 결정에 관여하지 않고 비즈니스 로직(인증, 변환 등)에만 집중한다.

### Routing (라우팅)
클라이언트 요청을 적절한 Upstream으로 전달하는 규칙. **경로 기반(path-based)**으로 동작하며, 규칙은 `nginx.conf`에 정적으로 정의된다.

### Header Manipulation (헤더 조작)
요청/응답의 HTTP 헤더를 추가·수정·제거하는 변환. 양방향(요청 → Upstream, Upstream → 클라이언트)으로 동작한다. 바디 변환보다 먼저 구현한다.

### Filter Config File (필터 설정 파일)
각 Wasm Filter의 설정(API Key 목록, Rate Limit 임계값 등)을 저장하는 JSON 파일. `config/<filter-name>.json` 경로에 필터별로 독립 관리한다. Rust의 `serde_json`으로 파싱.

### API Key Authentication (API Key 인증)
클라이언트가 `X-API-Key` 헤더로 Key를 전달하면, Wasm 필터가 Filter Config File(`config/api-key.json`)에 정의된 Key 목록과 대조해 검증한다. 검증 실패 시 `401 Unauthorized`를 반환하고 요청을 차단한다.

### Observability (관찰성)
게이트웨이의 동작을 외부에서 파악할 수 있게 하는 기능. 현재 범위: **구조화 JSON 로깅** (요청/응답 정보를 JSON 형식으로 출력). 메트릭(Prometheus), 트레이싱(OpenTelemetry)은 이후 단계.

### Rate Limiting (트래픽 제어)
클라이언트 요청 수를 제한해 업스트림 과부하를 방지하는 기능. API Key 기준으로 카운팅하며, 카운터는 Proxy-Wasm shared data API로 저장. nginx는 복수 worker로 운영 — shared data가 cross-worker로 공유되는지 검증 필요 (ngx_wasm_module 구현에 따라 정확도가 달라짐). 초과 시 `429 Too Many Requests` 반환.

### Body Transformation (바디 변환)
HTTP 바디를 검증하거나 포맷을 변환하는 작업. Wasm 필터 내부에서 바디 전문을 버퍼링해야 하므로 메모리 비용이 크다. Header Manipulation 이후 단계에서 구현 예정.

## Implementation language

- Wasm Filter: **Rust** (proxy-wasm-rust-sdk)
- Gateway 런타임: **ngx_wasm_module** (Kong, C)
