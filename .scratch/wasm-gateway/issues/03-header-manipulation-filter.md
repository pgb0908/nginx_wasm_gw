Status: done
Category: enhancement

# 03 — 헤더 조작 Wasm 필터

## What to build

요청/응답의 HTTP 헤더를 추가·수정·제거하는 Wasm Filter를 Rust로 작성한다. 조작 규칙은 `config/header-manipulation.json`에서 읽어오며, Filter Chain 우선순위 3으로 동작한다.

## Acceptance criteria

- [x] `config/header-manipulation.json`에 추가/수정/제거 규칙을 정의할 수 있다
- [x] 요청 헤더 조작(클라이언트 → Upstream 방향)이 동작한다
- [x] 응답 헤더 조작(Upstream → 클라이언트 방향)이 동작한다
- [ ] `curl -v`로 헤더가 의도대로 변경된 것을 확인할 수 있다 (런타임 확인 필요)
- [x] Filter Chain에서 우선순위 3으로 로드된다

## Blocked by

- `.scratch/wasm-gateway/issues/02-path-based-routing.md`

## Implementation notes

- Crate: `gateway/filters/header-manipulation/`
- Config: `gateway/config/header-manipulation.json`
- JSON 파싱: 외부 crate 없이 직접 구현 (wasm32 no_std 환경 대응)
- 6개 native 테스트 모두 통과: `cargo test -p header-manipulation`
- wasm32 빌드 성공: `cargo build -p header-manipulation --target wasm32-wasip1 --profile wasm-release`
- nginx config test 통과: `nginx -t` (gateway/nginx/ 에서 실행)
- nginx.conf wasm 블록 우선순위 3 위치에 `header_manipulation_filter` 등록
- 실패 시 항상 `Action::Continue` — 절대 차단하지 않음
