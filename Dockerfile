# syntax=docker/dockerfile:1.7

# Production build image for predictiq-api.
# Base image: rust:1.83-slim (digest verified on 2025-01-15).
# MSRV is declared in services/api/Cargo.toml (rust-version = "1.75");
# 1.83 is a current, supported toolchain that satisfies it.
# Base-image drift is tracked automatically via the Dependabot `docker`
# ecosystem entry in .github/dependabot.yml.
FROM rust:1.97-slim@sha256:8e8cf8f7fd54a2d23d5a743b3a03f56e26b6c774276c33fa0595111704ebb15c AS builder

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
FROM debian:bookworm-slim@sha256:0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/services/api/target/release/predictiq-api /usr/local/bin/predictiq-api

EXPOSE 8080

USER nobody

ENTRYPOINT ["predictiq-api"]

RUN cargo build --release --manifest-path services/api/Cargo.toml

# ---------- Runtime stage ----------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/services/api/target/release/predictiq-api /usr/local/bin/predictiq-api

EXPOSE 8080

USER nobody

ENTRYPOINT ["predictiq-api"]
