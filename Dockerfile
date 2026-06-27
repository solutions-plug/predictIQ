# Multi-stage build for PredictIQ API
# Pinned to specific digest for reproducible builds and security
# rust:1.75-slim digest verified on 2024-01-15
FROM rust:1.75-slim@sha256:4dd48afa1d6fcf622b18b60081bb6c897b11787b42006aea2f2cf5ff3f6ae0cc as builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY . .

# Build API service
RUN cd services/api && cargo build --release

# Runtime stage
# debian:bookworm-slim digest verified on 2024-01-15
FROM debian:bookworm-slim@sha256:3d868b89a1b0d8b957fa1798fffb5e1b6db5ac4e9c79e74acd418db9be3506b

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
# Prevents container escape vulnerabilities from granting root access to host
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Copy binary from builder
COPY --from=builder /build/services/api/target/release/predictiq-api /app/

# Set ownership to non-root user
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

CMD ["./predictiq-api"]
