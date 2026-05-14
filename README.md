# nginx-wasm-gw

**ngx_wasm_module(Proxy-Wasm)** 기반의 API 게이트웨이.  
선언적 JSON 설정 파일에서 nginx.conf를 자동 생성하고, Rust로 작성된 Wasm 필터 체인으로 API 트래픽을 처리한다.

---

## 아키텍처

```
┌─────────────────────────────────────────────────────────────────────┐
│                         gateway/                                    │
│                                                                     │
│  ┌──────────────────────┐       ┌─────────────────────────────────┐ │
│  │   gw-admin           │       │   nginx (ngx_wasm_module)       │ │
│  │   (Admin Process)    │       │                                 │ │
│  │                      │       │  ┌──────────────────────────┐   │ │
│  │  ┌────────────────┐  │ spawn │  │  Filter Chain (Proxy-Wasm│   │ │
│  │  │ config/        │  │──────▶│  │                          │   │ │
│  │  │  listeners/    │  │       │  │  ① api_key_filter        │   │ │
│  │  │  routers/      │  │ conf  │  │  ② rate_limit_filter     │   │ │
│  │  │  services/     │  │ regen │  │  ③ header_manip_filter   │   │ │
│  │  │  policies/     │  │◀──────│  │  ④ logging_filter        │   │ │
│  │  └────────────────┘  │ watch │  └──────────────────────────┘   │ │
│  │                      │       │             │                    │ │
│  │  ┌────────────────┐  │       │             ▼                    │ │
│  │  │ template.rs    │  │       │     proxy_pass upstream          │ │
│  │  │ (conf 생성)    │  │       └─────────────────────────────────┘ │
│  │  └────────────────┘  │                     │                    │
│  └──────────────────────┘                     │                    │
│                                               ▼                    │
│                                    ┌─────────────────┐             │
│                                    │  Backend Service │             │
│                                    │  svc-v1 :8081   │             │
│                                    │  svc-v2 :8082   │             │
│                                    └─────────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
```

### 구성 요소

| 구성 요소 | 위치 | 역할 |
|---|---|---|
| **gw-admin** | `gateway/admin/` | 설정 파싱 → nginx.conf 생성 → nginx 기동 → config 변경 감시 |
| **nginx** | `gateway/nginx/` | 요청 수신, Wasm 필터 체인 실행, 백엔드 프록시 |
| **Wasm 필터** | `gateway/filters/` | API Key 인증, Rate Limiting, 헤더 조작, JSON 로깅 |
| **Resource Model** | `gateway/config/` | Kubernetes 스타일 선언적 설정 (JSON) |

### 요청 처리 흐름

```
Client Request
      │
      ▼
 nginx :9000
      │
      ├─ location /api/v1/ ──▶ Filter Chain
      │                             │
      │                    ① api_key_filter      X-API-Key 검증 (401 반환)
      │                    ② rate_limit_filter   초당 요청 제한 (429 반환)
      │                    ③ header_manip_filter 요청/응답 헤더 조작
      │                    ④ logging_filter      JSON 형식 로그 출력
      │                             │
      │                    proxy_pass http://svc-v1
      │
      └─ location /api/v2/ ──▶ (동일 Filter Chain) ──▶ proxy_pass http://svc-v2
```

### gw-admin 내부 구조

```
gw-admin start
    │
    ├─ 1. config/ 읽기        GatewayConfig::load()
    │         Listener / Router / Service / Policy JSON 파싱
    │
    ├─ 2. nginx.conf 생성     template::render()
    │         Policy를 order 오름차순 정렬 후 proxy_wasm 지시어 나열
    │
    ├─ 3. nginx 기동          nginx::start()
    │         cwd=gateway/nginx, -p . -c nginx.conf
    │
    └─ 4. config 감시         watcher::start_watcher()  [백그라운드 스레드]
              gateway/config/ 변경 감지
                  → 재파싱 → nginx.conf 재생성 → nginx -s reload
              파싱 실패 시 reload 건너뜀 (기존 설정 유지)
```

---

## Resource Model

설정은 Kubernetes 스타일(`apiVersion: iip.gateway/v1alpha1`)의 JSON 파일로 작성한다.

```
gateway/config/
├── listeners/          # 수신 포트 정의
│   └── default-listener.json
├── routers/            # URL 경로 → Service 라우팅
│   ├── api-v1.json
│   └── api-v2.json
├── services/           # 백엔드 서버 그룹 (upstream)
│   ├── svc-v1.json
│   └── svc-v2.json
└── policies/           # 필터 정책 (order로 실행 순서 결정)
    ├── default-security.json   # order: 5  → api_key_filter
    ├── default-traffic.json    # order: 10 → rate_limit_filter
    └── default-transform.json  # order: 15 → header_manipulation_filter
```

**Listener 예시**:
```json
{
  "apiVersion": "iip.gateway/v1alpha1",
  "kind": "Listener",
  "metadata": { "name": "default-listener" },
  "spec": { "protocol": "HTTP", "port": 9000 }
}
```

**Policy 예시** (Security — API Key):
```json
{
  "apiVersion": "iip.gateway/v1alpha1",
  "kind": "Policy",
  "metadata": { "name": "default-security" },
  "spec": {
    "targetRef": { "kind": "Router", "name": "*" },
    "order": 5,
    "config": {
      "apiKey": {
        "header": "X-API-Key",
        "keys": ["secret-key-1", "secret-key-2"]
      }
    }
  }
}
```

Policy `targetRef.name: "*"` 는 모든 Router에 적용. 특정 Router 이름을 지정하면 해당 Router에만 적용된다.

---

## 요구 사항

| 항목 | 버전 |
|---|---|
| Rust | stable (rust-toolchain.toml 참조) |
| wasm32-wasip1 target | `rustup target add wasm32-wasip1` |
| just | `cargo install just` |
| Python 3 | 통합 테스트 mock upstream 용 |
| nginx (ngx_wasm_module) | `gateway/bin/nginx` 에 사전 배치 |

nginx 바이너리는 [Kong/ngx_wasm_module](https://github.com/kong/ngx_wasm_module) 릴리즈에서 다운로드해서 `gateway/bin/nginx`에 배치한다.

---

## 설치

```bash
# 1. 저장소 클론
git clone <repo-url>
cd nginx_wasm_gw

# 2. wasm32 타겟 추가
rustup target add wasm32-wasip1

# 3. Wasm 필터 빌드
just build

# 4. gw-admin 바이너리 빌드
just build-admin
```

---

## 실행

### 기본 실행

```bash
# nginx 시작 (config 읽기 → nginx.conf 생성 → nginx 기동 → config 감시)
just nginx

# 다른 터미널에서 nginx 중지
just stop
```

### gw-admin 직접 사용

```bash
# nginx 시작 (포그라운드, config 변경 자동 감시)
./target/debug/gw-admin start \
  --config-dir gateway/config \
  --nginx-prefix gateway/nginx \
  --nginx-bin gateway/bin/nginx

# nginx 중지
./target/debug/gw-admin stop

# nginx 상태 확인
./target/debug/gw-admin status

# config 수동 reload (nginx.conf 재생성 + nginx -s reload)
./target/debug/gw-admin reload

# nginx.conf만 생성 (stdout 출력)
./target/debug/gw-admin generate

# nginx.conf 파일로 저장
./target/debug/gw-admin generate --out gateway/nginx/nginx.conf
```

### Config 변경 반영

`gw-admin start` 실행 중에는 `gateway/config/` 를 자동으로 감시한다.  
JSON 파일을 수정하면 파싱 → nginx.conf 재생성 → `nginx -s reload` 가 자동으로 실행된다.

```bash
# 예: API Key 추가
# gateway/config/policies/default-security.json 편집
# → 저장하면 자동으로 nginx reload
```

파싱 실패 시에는 reload 없이 에러 로그만 출력하고 기존 설정을 유지한다.

### 통합 테스트

```bash
# Wasm 필터 빌드 + gw-admin 빌드 + 전체 12개 시나리오 실행
just integration
```

테스트 항목:
1. 경로 기반 라우팅 (`/api/v1/` → svc-v1, `/api/v2/` → svc-v2)
2. API Key 인증 (유효/무효 키, 키 없음)
3. 응답 헤더 조작 (추가/제거)
4. Rate Limiting (임계 초과 시 429)
5. JSON 구조 로그 출력

### 유닛 테스트

```bash
just test
```

---

## 프로젝트 구조

```
nginx_wasm_gw/
├── gateway/
│   ├── admin/                  # gw-admin 바이너리 (Rust)
│   │   └── src/
│   │       ├── main.rs         # CLI 진입점 (clap)
│   │       ├── config.rs       # Resource Model 파싱
│   │       ├── template.rs     # nginx.conf 생성
│   │       ├── nginx.rs        # nginx 프로세스 제어
│   │       └── watcher.rs      # config 파일 감시 (notify)
│   ├── filters/                # Wasm 필터 (Rust, wasm32-wasip1)
│   │   ├── api-key/            # API Key 인증
│   │   ├── rate-limit/         # Rate Limiting (인메모리 카운터)
│   │   ├── header-manipulation/ # 요청/응답 헤더 조작
│   │   ├── logging/            # JSON 구조 로그
│   │   └── passthrough/        # 패스스루 (기본 필터)
│   ├── config/                 # Resource Model JSON 설정
│   │   ├── listeners/
│   │   ├── routers/
│   │   ├── services/
│   │   └── policies/
│   ├── nginx/                  # nginx 런타임 디렉터리
│   │   └── logs/
│   ├── bin/                    # nginx 바이너리 (ngx_wasm_module)
│   └── scripts/
│       ├── integration-test.sh
│       └── mock-upstream.py
├── docs/
│   ├── architecture.md
│   ├── adr/                    # Architecture Decision Records
│   └── config-models/          # Resource Model 스펙 문서
├── Cargo.toml                  # Workspace 루트
├── rust-toolchain.toml
└── justfile                    # 빌드/실행 명령어
```

---

## 구현 이력

| 이슈 | 내용 |
|---|---|
| #01 | Passthrough 필터 스캐폴딩 및 Proxy-Wasm 빌드 파이프라인 구축 |
| #02 | 경로 기반 라우팅 (`/api/v1/`, `/api/v2/`) |
| #03 | Header Manipulation 필터 — 요청/응답 헤더 추가·제거 |
| #04 | API Key 인증 필터 — `X-API-Key` 헤더 검증, 실패 시 401 |
| #05 | JSON 로깅 필터 — 메서드·경로·상태코드를 JSON으로 error.log 출력 |
| #06 | Rate Limiting 필터 — 인메모리 슬라이딩 윈도우, 초과 시 429 |
| #07 | Wasm 필터 복원 — WASI filesystem 샌드박스 제약 발견, `get_plugin_configuration()` 방식으로 복원 |
| #08 | Admin Process 스캐폴딩 — `gw-admin` CLI 뼈대 + Kubernetes 스타일 Resource Model JSON 파싱 |
| #09 | nginx.conf 자동 생성 — Resource Model → upstream/server/location 블록 렌더링 |
| #10 | nginx Lifecycle 관리 — `gw-admin start/stop/status`, cwd 기반 wasm 경로 문제 해결 |
| #11 | Config 파일 감시 — `notify` 크레이트로 `gateway/config/` 감시, 변경 시 자동 reload |

### 핵심 설계 결정 (ADR)

- **ADR-0001**: Rate Limiting은 인메모리 카운터 사용 (Wasm 샌드박스 제약으로 외부 저장소 불가)
- **ADR-0002**: Admin Process가 nginx.conf를 생성 (Wasm 필터에서 파일 직접 읽기 불가 — WASI preopen 미지원)
- **ADR-0003**: Kubernetes 스타일 Resource Model — `apiVersion`, `kind`, `metadata`, `spec` 구조

---

## 기술 스택

| 기술 | 용도 |
|---|---|
| **nginx + ngx_wasm_module** | API 게이트웨이 코어 (Kong OSS) |
| **Proxy-Wasm** | 필터 체인 표준 인터페이스 |
| **Rust** | Wasm 필터 + gw-admin 구현 언어 |
| **wasm32-wasip1** | Wasm 컴파일 타겟 |
| **wasmtime** | nginx 내 Wasm 런타임 |
| **serde / serde_json** | Resource Model JSON 파싱 |
| **clap** | gw-admin CLI |
| **notify** | config 파일 변경 감시 |
