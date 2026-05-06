# ── Stage 1: Build static binary ──────────────────────────────────────────────
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependencies layer — copy manifests first, then source
COPY Cargo.toml ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
# Touch main.rs so Cargo knows to rebuild it (dummy above messed the mtime)
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Minimal runtime ───────────────────────────────────────────────────
FROM alpine:3.20

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/target/release/loco /usr/local/bin/loco

# MCP uses stdio — no ports exposed
ENTRYPOINT ["/usr/local/bin/loco"]
