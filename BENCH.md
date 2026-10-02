# Benchmarks — rust-server-starter

Measured 2026-10-02 on a 2-CPU / 8 GB Linux VM. Every number below was measured, not estimated.

## Toolchain

- `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1`
- axum **0.8.9**, tokio **1.53.1** (full features), tower-http 0.6, tracing-subscriber 0.3 (json fmt)
- `cargo build --release`, default profile (opt-level 3)

## Build

| Metric | Value | How measured |
|---|---|---|
| From-scratch release build | **187 s wall** (3m07s) | `cargo clean` then `time cargo build --release`; `real 3m7.016s` |
| Release binary | **3,961,008 bytes (3.78 MiB)** | `stat -c%s target/release/rust-server-starter` |

## Runtime footprint

| Metric | Value | How measured |
|---|---|---|
| Idle RSS | **3,484 kB (3.4 MB)** | Ran release binary, waited 15 s with zero traffic, read `VmRSS` from `/proc/<PID>/status` |

## Load test

Tool: `loadtest.py` in this repo — python3, `concurrent.futures`, 50 worker threads, one persistent HTTP/1.1 keep-alive connection per thread, 15 s per endpoint. Latency measured client-side per request.

| Endpoint | req/s | p50 | p99 | requests | errors |
|---|---|---|---|---|---|
| `GET /api/hello?name=load` | **2,380** | **17.5 ms** | **78.9 ms** | 35,754 | 0 |
| `POST /api/echo` (52-byte JSON body) | **3,146** | **13.3 ms** | **49.5 ms** | 47,245 | 0 |

Caveats (honest):

- The server logs **every request as JSON to stdout** and generates a **UUID v4 per request** — that overhead is included in these numbers, as it would be in production with this config.
- The load generator is 50 Python threads on a 2-CPU VM; part of the p50/p99 latency is client-side thread scheduling, not server time. Throughput (req/s) is the more robust figure.

## Docker

**Skipped — docker is not installed on this VM** (`which docker` → not found). The `Dockerfile` (multi-stage `rust:1-bookworm` → `debian:bookworm-slim`, non-root `appuser`) is written but the image was never built, so no image size is reported. Build it on a machine with docker via the README instructions.

## Route verification (actual curl output)

```
$ curl -s localhost:3000/health
{"status":"ok"}

$ curl -s 'localhost:3000/api/hello?name=luke'
{"message":"hello, luke"}

$ curl -s localhost:3000/api/hello
{"message":"hello, world"}

$ curl -s -X POST localhost:3000/api/echo -H 'content-type: application/json' -d '{"a":1,"b":[2,3]}'
{"a":1,"b":[2,3]}

$ curl -s localhost:3000/version
{"version":"0.1.0"}

$ curl -s -o /dev/null -w "%{http_code} %{size_download} bytes\n" localhost:3000/
200 654 bytes

$ curl -s -w "\n%{http_code}\n" localhost:3000/nope
{"error":"not found"}
404
```

`x-request-id` response header present on all responses. `SIGTERM` → logs `shutdown signal received`, exits cleanly (verified).
