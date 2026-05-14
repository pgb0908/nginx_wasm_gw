Status: done
Category: enhancement

# 15 — nginx.conf 로깅 설정 동적 렌더링

## What to build

`template.rs`의 `error_log` 하드코딩을 제거하고 Gateway 리소스의 `spec.logging` 설정을 읽어 동적으로 렌더링한다. http 블록에 `log_format` 정의와 `access_log` 지시어를 추가한다. `gateway/config/gateways/default.json` 예시 파일을 함께 추가한다.

nginx.conf에 생성되는 결과:

```nginx
# error_log — Gateway.spec.logging.errorLog.level 사용 (기본 info)
error_log logs/error.log info;

# http 블록 안 — accessLog.format에 따라 log_format 정의
log_format gw_json escape=json '{"time":"$time_iso8601","method":"$request_method","uri":"$request_uri","status":$status,"bytes":$body_bytes_sent}';
access_log logs/access.log gw_json;

# accessLog.enabled: false 이면
access_log off;
```

Gateway 리소스가 없으면 기본값(`errorLog.level: info`, `accessLog.enabled: true`, `accessLog.format: JSON`)으로 동작한다.

## Acceptance criteria

- [ ] `template::render()`가 `GatewayConfig.gateway`를 받아 `error_log` 레벨을 동적으로 렌더링한다
- [ ] `accessLog.enabled: true`이면 http 블록에 `log_format` + `access_log` 지시어가 생성된다
- [ ] `accessLog.format: JSON`이면 `gw_json` 포맷, `TEXT`이면 `gw_text` 포맷이 사용된다
- [ ] `accessLog.enabled: false`이면 `access_log off;`가 생성된다
- [ ] Gateway 리소스 없음 → 기존 동작과 동일 (error_log info, access_log gw_json)
- [ ] `gateway/config/gateways/default.json` 예시 파일이 추가된다
- [ ] 기존 통합 테스트 12/12가 유지된다

## Blocked by

- 14-gateway-resource-parsing.md
