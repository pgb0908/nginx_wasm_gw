# ADR-0002: Admin Process가 nginx.conf를 생성하고 nginx를 관리한다

## Status

Accepted

## Context

Wasm 필터에 JSON config를 전달하는 방법을 결정해야 했다. Proxy-Wasm의 표준 메커니즘은 `get_plugin_configuration()`이며, 이는 nginx.conf의 `proxy_wasm filter_name '...'` 인라인 JSON을 읽는다.

초기 시도: wasm 필터가 `std::fs::read_to_string`으로 `gateway/config/*.json`을 직접 읽으려 했으나, ngx_wasm_module의 wasmtime 샌드박스가 WASI filesystem preopen을 허용하지 않아 실패했다.

## Decision

별도의 Rust 데몬(`gw-admin`, `gateway/admin/` 크레이트)이 다음을 담당한다:

1. `gateway/config/*.json` 파일을 읽어 Rust 구조체로 파싱
2. 파싱된 config로 `gateway/nginx/nginx.conf`를 생성 (Rust format string 기반)
3. nginx를 child process로 spawn하고 생명주기를 관리
4. `gateway/config/` 디렉토리를 watch하다가 변경 시 nginx.conf 재생성 + `nginx -s reload`

Wasm 필터는 `get_plugin_configuration()`으로 config를 수신하며 파일 접근을 하지 않는다. `gateway/nginx/nginx.conf`는 generated file로 gitignore에 추가한다.

## Consequences

- **Filter Config File이 source of truth**: `gateway/config/*.json`을 편집하면 자동으로 반영된다
- **nginx.conf는 직접 편집하지 않는다**: 라우팅·upstream 변경도 Admin Process와 템플릿 코드를 통해야 한다
- **추후 Admin REST API 추가 시**: `gw-admin`에 HTTP 포트를 붙여 실행 중인 config를 동적으로 변경 가능 (nginx -s reload 없이)
- **단점**: nginx.conf 구조 변경 시 `gateway/admin/src/template.rs`를 수정해야 한다
