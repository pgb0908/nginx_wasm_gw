Status: done
Category: enhancement

# 06 — Rate Limiting Wasm 필터 + cross-worker 검증

## What to build

API Key 기준으로 요청 수를 카운팅해 임계값 초과 시 `429 Too Many Requests`를 반환하는 Wasm Filter를 Rust로 작성한다. 카운터는 Proxy-Wasm shared data API에 저장한다. Filter Chain 우선순위 1로 가장 먼저 실행된다.

nginx는 복수 worker로 운영하므로, shared data가 cross-worker로 실제 공유되는지 실측 검증이 필요하다 (ADR-0001 참고).

## Acceptance criteria

- [ ] `config/rate-limit.json`에 API Key별 임계값(예: 초당 N회)을 정의할 수 있다
- [ ] 임계값 이하 요청은 정상 처리된다
- [ ] 임계값 초과 요청은 `429 Too Many Requests`를 반환한다
- [ ] Filter Chain에서 우선순위 1로 로드된다 (가장 먼저 실행)
- [ ] 복수 worker 환경(`worker_processes 2` 이상)에서 부하를 주어 shared data가 cross-worker로 공유되는지 확인한다 — per-worker라면 결과를 문서화하고 ADR-0001을 업데이트한다

## Blocked by

- `.scratch/wasm-gateway/issues/02-path-based-routing.md`

## See also

- `docs/adr/0001-rate-limiting-in-memory-counter.md`
