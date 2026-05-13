Status: done

# 05 — 구조화 JSON 로깅 Wasm 필터

## What to build

요청/응답의 메타데이터를 JSON 형식으로 출력하는 Wasm Filter를 Rust로 작성한다. Filter Chain 우선순위 4로 동작해 다른 필터들의 처리 결과(인증 결과, 변환된 헤더 등)가 반영된 이후 로깅한다.

로깅 항목 예시: HTTP 메서드, 요청 경로, 상태 코드, 응답 지연시간, API Key 식별자.

## Acceptance criteria

- [ ] 요청마다 JSON 형식의 로그 한 줄이 출력된다
- [ ] 로그에 메서드, 경로, 상태 코드, 지연시간이 포함된다
- [ ] Filter Chain에서 우선순위 4로 로드된다 (가장 마지막)

## Blocked by

- `.scratch/wasm-gateway/issues/02-path-based-routing.md`
