# ADR-0004: 릴리즈는 nginx를 포함한 self-contained tar.gz 번들로 배포한다

## Status

Accepted

## Context

nginx-wasm-gw는 세 종류의 아티팩트로 구성된다: Wasm 필터(`.wasm`), Admin Process(`gw-admin`), 그리고 nginx 바이너리. 이 중 nginx는 표준 nginx가 아니라 Kong이 유지하는 [ngx_wasm_module](https://github.com/kong/ngx_wasm_module) 빌드다.

사용자가 릴리즈를 설치할 때 선택지는 두 가지였다:

1. **nginx 미포함** — 사용자가 직접 ngx_wasm_module 버전을 맞춰 빌드하거나 다운로드
2. **nginx 포함** — 모든 구성 요소를 하나의 아카이브에 번들링

## Decision

릴리즈는 **nginx를 포함한 self-contained `tar.gz` 아카이브**로 배포한다.

### 번들 구조

```
nginx-wasm-gw-<version>-linux-x86_64.tar.gz
└── nginx-wasm-gw/
    ├── bin/
    │   ├── gw-admin          ← --release 빌드된 Admin Process
    │   └── nginx             ← ngx_wasm_module 바이너리
    ├── filters/              ← Wasm 필터 (wasm32-wasip1, wasm-release)
    │   ├── api_key.wasm
    │   ├── rate_limit.wasm
    │   ├── header_manipulation.wasm
    │   ├── logging.wasm
    │   └── passthrough.wasm
    ├── nginx/                ← nginx prefix 디렉토리 (cwd)
    │   ├── logs/
    │   └── tmp/
    │       ├── client_body/
    │       ├── proxy/
    │       ├── fastcgi/
    │       ├── scgi/
    │       └── uwsgi/
    ├── config/               ← 예시 Resource Model JSON
    │   ├── listeners/
    │   ├── routers/
    │   ├── services/
    │   └── policies/
    └── README.md
```

### Wasm 경로 규칙

nginx가 `nginx/` 디렉토리를 cwd로 실행되므로, 생성된 nginx.conf의 wasm 모듈 경로는 `../filters/<name>.wasm`이 된다. `gw-admin`의 `--wasm-dir` 인자(기본값 `../../target/wasm32-wasip1/wasm-release`)로 경로를 제어하며, 릴리즈 환경에서는 `--wasm-dir ../filters`로 지정한다.

### 버전 결정

`git tag` (예: `v1.0.0`)를 기준으로 파일명을 결정한다. 태그가 없으면 `v0.0.0-dev`로 fallback한다.

### 빌드 트리거

`just release` 명령으로 로컬에서 생성한다. CI 자동화는 별도 이슈로 추후 추가한다.

### 배포 대상 플랫폼

**Linux x86_64 단일 플랫폼**. arm64, macOS 지원은 별도 이슈로 추후 추가한다.

## Consequences

- 사용자는 tar.gz를 내려받아 압축을 풀고 `gw-admin start`만 실행하면 된다.
- 번들 크기가 증가하지만(nginx 바이너리 포함), 설치 절차가 단순해진다.
- ngx_wasm_module 버전이 릴리즈에 고정되므로 버전 호환성 문제가 없다.
- `gw-admin`의 `--wasm-dir` 인자가 개발 환경과 릴리즈 환경 간 경로 차이를 흡수한다.
