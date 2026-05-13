Status: done
Category: enhancement

# 01 — 개발 환경 + 빈 Passthrough 필터

## What to build

ngx_wasm_module 공식 릴리즈에서 미리 빌드된 nginx 바이너리를 받아 로컬에서 직접 실행하는 개발 환경을 구성한다. Rust workspace와 proxy-wasm-rust-sdk를 설정하고, 아무 로직 없이 요청을 그대로 통과시키는 빈 Wasm Filter를 작성해 nginx에 로드한다.

바이너리 명명 규칙: `wasmx-{version}-{runtime}-linux-x86_64.tar.gz` (GitHub Releases에서 다운로드).

## Acceptance criteria

- [ ] `wasmx` nginx 바이너리를 로컬에서 실행할 수 있다
- [ ] Rust workspace가 설정되어 있고 `cargo build --target wasm32-wasi`로 `.wasm` 파일이 생성된다
- [ ] 빈 passthrough 필터가 `nginx.conf`에 로드된다
- [ ] `curl localhost`로 요청 시 nginx가 정상 응답한다 (필터가 요청을 차단하지 않음)

## Blocked by

None — 즉시 시작 가능
