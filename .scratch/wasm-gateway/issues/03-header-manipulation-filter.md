Status: ready-for-agent

# 03 — 헤더 조작 Wasm 필터

## What to build

요청/응답의 HTTP 헤더를 추가·수정·제거하는 Wasm Filter를 Rust로 작성한다. 조작 규칙은 `config/header-manipulation.json`에서 읽어오며, Filter Chain 우선순위 3으로 동작한다.

## Acceptance criteria

- [ ] `config/header-manipulation.json`에 추가/수정/제거 규칙을 정의할 수 있다
- [ ] 요청 헤더 조작(클라이언트 → Upstream 방향)이 동작한다
- [ ] 응답 헤더 조작(Upstream → 클라이언트 방향)이 동작한다
- [ ] `curl -v`로 헤더가 의도대로 변경된 것을 확인할 수 있다
- [ ] Filter Chain에서 우선순위 3으로 로드된다

## Blocked by

- `.scratch/wasm-gateway/issues/02-path-based-routing.md`
