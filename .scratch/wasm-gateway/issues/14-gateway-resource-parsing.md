Status: done
Category: enhancement

# 14 — Gateway 리소스 파싱

## What to build

`config.rs`에 `Gateway` 리소스 구조체를 추가하고 `gateway/config/gateways/` 디렉토리에서 로딩한다. Gateway는 전역 단일 설정이므로 파일이 없으면 `None`(기본값 fallback), 2개 이상이면 로딩 실패로 처리한다. `GatewayConfig`에 `gateway: Option<Gateway>` 필드를 추가해 이후 렌더링 레이어가 사용할 수 있게 한다.

Gateway JSON 구조 (from gateway.md):

```json
{
  "apiVersion": "iip.gateway/v1alpha1",
  "kind": "Gateway",
  "metadata": { "name": "default-gateway" },
  "spec": {
    "logging": {
      "errorLog": { "level": "info" },
      "accessLog": { "enabled": true, "format": "JSON" }
    }
  }
}
```

## Acceptance criteria

- [ ] `GatewayConfig`에 `gateway: Option<Gateway>` 필드가 추가된다
- [ ] `gateways/` 디렉토리에 파일이 없으면 `gateway: None`으로 로딩 성공한다
- [ ] `gateways/` 디렉토리에 파일이 2개 이상이면 `GatewayConfig::load()`가 Err를 반환한다
- [ ] `spec.logging.errorLog.level`이 파싱된다 (기본값 `"info"`)
- [ ] `spec.logging.accessLog.enabled`가 파싱된다 (기본값 `true`)
- [ ] `spec.logging.accessLog.format`이 `"JSON"` 또는 `"TEXT"`로 파싱된다 (기본값 `"JSON"`)
- [ ] 유닛 테스트: 파일 없음 → `None`, 유효한 파일 → 파싱 성공, 파일 2개 → Err

## Blocked by

None — 즉시 시작 가능
