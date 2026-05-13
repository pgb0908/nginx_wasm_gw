Status: ready-for-agent

# 02 — 경로 기반 라우팅 + Upstream 블록

## What to build

`nginx.conf`에 `upstream` 블록과 경로별 `proxy_pass`를 정의해 경로 기반 라우팅을 구성한다. Wasm Filter는 라우팅 결정에 관여하지 않고 nginx가 직접 담당한다.

## Acceptance criteria

- [ ] `nginx.conf`에 최소 2개의 `upstream` 블록이 정의된다
- [ ] 경로 `/api/v1/...` → upstream A, `/api/v2/...` → upstream B 형태로 라우팅된다
- [ ] `curl`로 각 경로에 요청 시 지정한 upstream에 도달하는 것을 확인할 수 있다
- [ ] 테스트용 upstream은 로컬 mock 서버(예: `httpbin`) 또는 간단한 `python -m http.server`로 대체 가능하다

## Blocked by

- `.scratch/wasm-gateway/issues/01-scaffold-passthrough-filter.md`
