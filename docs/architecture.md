

이 프로젝트는 ngx_wasm_module(Proxy-Wasm)을 활용한 gateway proxy 서버를 만드는게 목표 

ngx_wasm_module은 nginx에 wasm을 추가한 버전이다.
nginx 요청처리에 wasm 필터를 적용한 것으로
요청 처리에 추가 로직을 wasm 샌드박스에 로딩하여 사용한다.

```text
ginx (Igor Sysoev, 2004)
                    │
                    │  원본 nginx 코어 (C)
                    │
        ┌───────────┼───────────────┐
        │           │               │
        ▼           ▼               ▼
   OpenResty    ngx_wasm_module   wasm-nginx-module
   (章亦春)      (Kong/Thibault)   (API7/APISIX)
    2007~         2022~             2021~
```

ngx_wasm_module은 그림과 같이 kong에서 만든 오픈소스이다.
https://github.com/kong/ngx_wasm_module << ngx_wasm_module의 git 링크


wasm의 개발 언어
wasm에 사용하는 개발 언어는 rust 사용한다.
부가적인 다른 기능을 개발할 때 rust를 사용한다.

