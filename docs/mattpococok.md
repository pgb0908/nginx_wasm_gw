Matt Pocock의 스킬들은 소프트웨어 개발 워크플로우 전체를 단계별로 커버하도록 설계


```text
  아이디어
    │
    ▼
  /grill-me          ─── "이 설계 괜찮아? 허점 찾아봐"
    │
    ▼
  /grill-with-docs   ─── "기존 코드/문서랑 충돌 없어?"
    │                     (CONTEXT.md, docs/adr/ 읽음)
    ▼
  /to-prd            ─── 대화 내용 → PRD 문서로 이슈 생성
    │
    ▼
  /to-issues         ─── PRD → 독립적인 작은 이슈들로 분해
    │
    ▼
  /triage            ─── 이슈 상태 관리 (needs-triage → ready-for-agent 등)
    │
    ▼
  /tdd               ─── 각 이슈를 TDD로 구현 (red→green→refactor)
    │
    ▼
  /diagnose          ─── 버그 발생 시 체계적 디버깅
    │
    ▼
  /improve-codebase-architecture  ─── 전체 구조 개선 기회 탐색
```

```text
  - CONTEXT.md — 프로젝트 도메인 언어 (용어 통일)
  - docs/adr/ — 과거 아키텍처 결정들
  - docs/agents/issue-tracker.md — 이슈 저장 위치 (방금 설정한 것)
  - docs/agents/triage-labels.md — 이슈 상태 이름
```