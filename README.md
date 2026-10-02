# rust-server-starter

A minimal, production-shaped HTTP server in Rust: [Axum](https://github.com/tokio-rs/axum) + Tokio + structured JSON logs, request-id middleware, static file serving, and graceful shutdown. A sane starting point for a default server language.

## Quickstart

```sh
cargo run
# listening on 0.0.0.0:3000
```

Set the port with `PORT`:

```sh
PORT=8080 cargo run
```

Log level with `RUST_LOG` (default `rust_server_starter=info`):

```sh
RUST_LOG=debug cargo run
```

## Routes

```sh
curl localhost:3000/health
# {"status":"ok"}

curl 'localhost:3000/api/hello?name=luke'
# {"message":"hello, luke"}

curl localhost:3000/api/hello
# {"message":"hello, world"}

curl -X POST localhost:3000/api/echo \
  -H 'content-type: application/json' \
  -d '{"a":1,"b":[2,3]}'
# {"a":1,"b":[2,3]}

curl localhost:3000/version
# {"version":"0.1.0"}

curl localhost:3000/
# serves ./static/index.html

curl -i localhost:3000/api/hello
# ... x-request-id: <uuid> ...
```

Every request logs a JSON line with a request id, method, path, status, and latency in ms.

## Docker

```sh
docker build -t rust-server-starter .
docker run --rm -p 3000:3000 -e PORT=3000 rust-server-starter
```

Multi-stage build: `rust:1-bookworm` compiles, `debian:bookworm-slim` runs the binary as a non-root user.

## Layout

```
src/main.rs   # routes, middleware, graceful shutdown
static/       # served at / (index.html at /)
Dockerfile    # multi-stage, non-root, slim runtime
```
