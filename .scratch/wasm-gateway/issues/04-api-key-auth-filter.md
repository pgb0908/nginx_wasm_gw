Status: done
Category: enhancement

# 04 — API Key 인증 Wasm 필터

## What to build

클라이언트가 `X-API-Key` 헤더로 전달한 Key를 `config/api-key.json`의 허용 목록과 대조해 검증하는 Wasm Filter를 Rust로 작성한다. Filter Chain 우선순위 2로 동작해 인증 실패 시 요청이 Upstream에 도달하기 전에 차단된다.

## Acceptance criteria

- [ ] `config/api-key.json`에 허용 API Key 목록을 정의할 수 있다
- [ ] 유효한 `X-API-Key` 헤더가 있으면 요청이 Upstream에 전달된다
- [ ] 헤더가 없거나 Key가 목록에 없으면 `401 Unauthorized`를 반환한다
- [ ] Filter Chain에서 우선순위 2로 로드된다 (Rate Limiting 다음, 헤더 조작 이전)

## Blocked by

- `.scratch/wasm-gateway/issues/02-path-based-routing.md`
