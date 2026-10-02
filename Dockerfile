# ---- builder ----
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Cache dependencies: build a dummy crate first so later source changes
# don't re-download/recompile all deps.
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release && rm -rf src
COPY src ./src
COPY static ./static
RUN cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim
RUN useradd -r -u 10001 -m -s /usr/sbin/nologin appuser
WORKDIR /app
COPY --from=builder /app/target/release/rust-server-starter /usr/local/bin/app
COPY --from=builder /app/static ./static
USER appuser
EXPOSE 3000
ENV PORT=3000
ENTRYPOINT ["/usr/local/bin/app"]
