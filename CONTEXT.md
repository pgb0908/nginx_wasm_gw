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

### Resource Model (리소스 모델)
게이트웨이 설정을 표현하는 선언형 단위. Kubernetes 스타일(`apiVersion`, `kind`, `metadata`, `spec`)을 따르며 `iip.gateway/v1alpha1` 버전을 사용한다. `gateway/config/<kind>/` 디렉토리에 리소스별로 JSON 파일로 저장한다. Admin Process가 이 파일들을 읽어 nginx.conf를 생성한다. 스펙은 `docs/config-models/`에 정의돼 있다.

### Listener
게이트웨이가 외부 트래픽을 수신할 포트·프로토콜·TLS를 정의하는 리소스. nginx의 `server { listen }` 블록으로 렌더링된다.

### Router
Listener로 들어온 트래픽을 경로·메서드·헤더 기준으로 매칭해 Service로 전달하는 라우팅 규칙. nginx의 `location` 블록으로 렌더링된다. `spec.targetRef`로 Listener를 참조하고, `spec.config.destinations`로 Service를 지정한다.

### Service
백엔드 서버 클러스터를 정의하는 리소스. nginx의 `upstream` 블록으로 렌더링된다. 로드밸런싱·헬스체크·retry·circuit breaker를 포함한다.

### Policy
Router에 부착되는 횡단 관심사 처리 단위. `spec.order` 값으로 실행 순서가 결정되며, Admin Process가 order 기준으로 정렬해 `proxy_wasm` 지시어를 생성한다. `spec.rules`로 Router 범위 내 서브셋 경로/메서드에만 적용할 수 있다. 타입: Security(order 5), Traffic(order 10), Enhance(order 12), Transform(order 15).

### Policy - Security
인증·인가·접근 제어를 담당하는 Policy. `apiKey`(헤더 기반 Key 인증), `jwtValidation`(JWT 토큰 검증), `ipFilter`(CIDR 기반 접근 제어), `cors`(브라우저 접근 제어)를 포함한다. `apiKey.keys`가 현재 구현된 api_key_filter에 매핑된다.

### Policy - Traffic
트래픽 속도·동시성 제어를 담당하는 Policy. `rateLimit`(quota + burst), `maxConcurrency`, `slaTiers`(등급별 차등 적용)를 포함한다. rate_limit_filter에 매핑된다.

### Policy - Transform
헤더·쿼리·바디를 변환하는 Policy. `headerControl`(추가/제거), `queryControl`(rename/remove), `bodyTransformation`(포맷 변환), `dataMasking`을 포함한다. header_manipulation_filter에 매핑된다.

### Admin Process (관리 프로세스)
nginx의 생명주기와 Resource Model을 관리하는 Rust 데몬. `gateway/admin/` 크레이트로 구현하며 `gw-admin` 바이너리로 배포된다. `gateway/config/` 디렉토리의 리소스 파일을 읽어 nginx.conf를 생성하고, nginx를 child process로 소유·관리한다. 파일 변경 감지 시 nginx.conf를 재생성하고 `nginx -s reload`를 실행한다. 추후 Admin REST API로 실행 중인 config를 동적으로 변경하는 기능을 추가한다.

### API Key Authentication (API Key 인증)
클라이언트가 `X-API-Key` 헤더로 Key를 전달하면, Wasm 필터가 Key 목록과 대조해 검증한다. Source of truth는 `gateway/config/policies/<name>.json`의 `spec.config.apiKey.keys`이며, Admin Process가 nginx.conf에 주입한다. 검증 실패 시 `401 Unauthorized`를 반환하고 요청을 차단한다.

### Observability (관찰성)
게이트웨이의 동작을 외부에서 파악할 수 있게 하는 기능. 현재 범위: **구조화 JSON 로깅** (요청/응답 정보를 JSON 형식으로 출력). 메트릭(Prometheus), 트레이싱(OpenTelemetry)은 이후 단계.

### Rate Limiting (트래픽 제어)
클라이언트 요청 수를 제한해 업스트림 과부하를 방지하는 기능. API Key 기준으로 카운팅하며, 카운터는 worker별 `thread_local` 메모리에 저장 (ADR-0001). 임계값은 `gateway/config/rate-limit.json`에서 Admin Process가 읽어 nginx.conf에 주입한다. 초과 시 `429 Too Many Requests` 반환.

### Body Transformation (바디 변환)
HTTP 바디를 검증하거나 포맷을 변환하는 작업. Wasm 필터 내부에서 바디 전문을 버퍼링해야 하므로 메모리 비용이 크다. Header Manipulation 이후 단계에서 구현 예정.

### Gateway (게이트웨이 전역 설정)
게이트웨이 전체에 적용되는 전역 설정을 정의하는 리소스. `gateway/config/gateways/` 디렉토리에 JSON 파일 하나로 존재하며, 파일이 없으면 기본값으로 동작한다. 두 개 이상이면 시작 실패. 현재 구현 범위: `spec.logging.errorLog.level`(nginx error_log 레벨, 기본 `info`)과 `spec.logging.accessLog`(enabled 기본 `true`, format `JSON`|`TEXT` 기본 `JSON`). accessLog format은 gw-admin이 내장한 nginx log_format 패턴(`gw_json`, `gw_text`)으로 렌더링된다.

### Release Bundle (릴리즈 번들)
배포 단위. nginx 바이너리(ngx_wasm_module), Wasm 필터, gw-admin 바이너리, 예시 Resource Model JSON을 하나의 `tar.gz` 아카이브에 묶은 self-contained 패키지. 파일명 형식: `nginx-wasm-gw-<version>-linux-x86_64.tar.gz`. 버전은 git tag에서 결정하며 태그 없으면 `v0.0.0-dev`로 fallback. `just release`로 로컬 생성한다. 배포 대상은 Linux x86_64 (ADR-0004).

### Wasm Dir (Wasm 디렉토리)
gw-admin이 nginx.conf의 wasm 모듈 경로를 생성할 때 사용하는 기준 디렉토리. nginx의 cwd(nginx prefix)를 기준으로 하는 상대 경로다. 개발 환경 기본값: `../../target/wasm32-wasip1/wasm-release`. 릴리즈 번들 환경 기본값: `../filters`. `gw-admin`의 `--wasm-dir` 인자로 지정한다.

## Implementation language

- Wasm Filter: **Rust** (proxy-wasm-rust-sdk)
- Gateway 런타임: **ngx_wasm_module** (Kong, C)
