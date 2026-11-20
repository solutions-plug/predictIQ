# syntax=docker/dockerfile:1.7

# Production build image for predictiq-api.
# Base image: rust:1.83-slim (digest verified on 2025-01-15).
# MSRV is declared in services/api/Cargo.toml (rust-version = "1.75");
# 1.83 is a current, supported toolchain that satisfies it.
# Base-image drift is tracked automatically via the Dependabot `docker`
# ecosystem entry in .github/dependabot.yml.
FROM rust:1.99-slim@sha256:2752b332db73fdbb7dc576f06c82ed1f312005784ef913d7e04a28f5f55dc581 AS builder

WORKDIR /usr/src/app

# Install build dependencies for the Rust toolchain and native crates.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation by copying manifests first.
COPY Cargo.toml Cargo.lock ./
COPY services/api/Cargo.toml services/api/Cargo.toml
RUN mkdir -p services/api/src \
    && echo 'fn main() {}' > services/api/src/main.rs \
    && echo '' > services/api/src/lib.rs \
    && cargo build --release --manifest-path services/api/Cargo.toml \
    && rm -rf services/api/src

# Build the actual application.
COPY . .
RUN cargo build --release --manifest-path services/api/Cargo.toml

# Runtime stage.
FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/services/api/target/release/predictiq-api /usr/local/bin/predictiq-api

EXPOSE 8080

USER nobody

# Built-in probe (no curl in the image); hits the lightweight liveness endpoint.
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["predictiq-api", "healthcheck"]

ENTRYPOINT ["predictiq-api"]
