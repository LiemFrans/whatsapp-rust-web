# ── Stage 1: Build ──────────────────────────────────────────────
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static perl make

WORKDIR /app

# Cache dependencies – copy manifests first
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy database migrations (needed by include_str! at compile time)
COPY database /database

# Build the real application
COPY backend/ .
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime ───────────────────────────────────────────
FROM alpine:3.20

RUN apk add --no-cache ca-certificates curl

WORKDIR /app

COPY --from=builder /app/target/release/server /app/server

EXPOSE 8080

ENV RUST_LOG=info,whatsapp_web_backend=debug

CMD ["/app/server"]
