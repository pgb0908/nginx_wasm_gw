# ADR-0001: Rate Limiting 카운터를 Proxy-Wasm Shared Data로 저장

## Status
Accepted

## Context
Rate Limiting 구현 시 카운터 저장소를 선택해야 했다. 선택지:

1. **Proxy-Wasm shared data API** — 외부 의존성 없음. cross-worker 공유 여부는 ngx_wasm_module 구현에 의존
2. **Redis** — 정확한 distributed counter. 외부 인프라 필요
3. **Per-worker 로컬 메모리** — 단순하지만 worker 수만큼 limit이 희석됨

초기 목표가 "부하를 얼마나 감당할 수 있는지 확인"이므로, 외부 의존성 없이 빠르게 검증할 수 있는 방식을 선택했다.

## Decision
Proxy-Wasm shared data API를 사용한다. nginx는 복수 worker(`worker_processes auto`)로 운영한다.

## Consequences
- **검증 필요:** ngx_wasm_module의 shared data가 실제로 cross-worker shared memory zone으로 구현되어 있는지 확인해야 한다. per-worker라면 Rate Limit 정확도가 worker 수만큼 희석된다.
- **확장 경로:** 프로덕션 수준의 정확한 Rate Limiting이 필요해지면 Redis 기반 counter로 교체한다.
- Redis 등 외부 의존성이 없어 로컬 개발 및 부하 테스트 환경 구성이 단순하다.
