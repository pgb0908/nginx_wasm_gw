# ADR-0003: Kubernetes 스타일 선언형 리소스 모델을 config로 채택한다

## Status

Accepted

## Context

게이트웨이 설정을 표현하는 방식을 결정해야 했다. 초기 구현은 필터별 flat JSON 파일(`rate-limit.json`, `api-key.json` 등)을 사용했으나, Admin Process가 Listener/Router/Service/Policy 전체를 관리하려면 더 구조적인 모델이 필요했다.

## Decision

모든 리소스는 `iip.gateway/v1alpha1` apiVersion과 Kubernetes 스타일(`apiVersion`, `kind`, `metadata`, `spec`)을 따른다. 파일은 `gateway/config/<kind>/` 디렉토리에 리소스별로 저장한다:

```
gateway/config/
├── gateways/
├── listeners/
├── routers/
├── services/
└── policies/
```

리소스 관계: `Policy.spec.targetRef` → Router, `Router.spec.targetRef` → Listener, `Router.spec.config.destinations[].destinationRef` → Service.

Policy 실행 순서는 `spec.order`로 결정한다 (Security:5, Traffic:10, Enhance:12, Transform:15). Admin Process가 order 기준으로 정렬해 `proxy_wasm` 지시어를 생성한다.

`policyRef` 필드는 제거한다. 설정은 `spec.config`에 인라인으로 정의한다.

`policy-security`에 `apiKey` 타입을 추가해 현재 api_key_filter와 매핑한다.

## Consequences

- **선언형**: 원하는 상태를 기술하면 Admin Process가 nginx.conf로 변환한다
- **확장성**: 새 리소스 타입·Policy 타입 추가 시 디렉토리와 파싱 코드만 추가
- **단점**: flat JSON보다 파일 수가 많아지고 리소스 간 참조 관계를 Admin Process가 검증해야 한다
