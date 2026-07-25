# Multi-stage build for PredictIQ TTS Service
# Pinned to Node.js 20 LTS (matching the "engines" field in package.json)
# node:20-alpine (20.19.0) digest verified on 2025-07-25
# Index digest covers linux/amd64 and linux/arm64 manifests

# ── Stage 1: Builder ─────────────────────────────────────────────────────────
FROM node:20-alpine@sha256:658d0f63e501824d6c23e06d4bb95c71e7d704537c9d9272f488ac03a370d448 AS builder

WORKDIR /build

# Copy dependency manifests first for layer caching
COPY package.json package-lock.json ./

# Install all dependencies (including devDependencies needed for tsc)
RUN npm ci

# Copy source and compile TypeScript
COPY tsconfig.json ./
COPY src ./src
RUN npm run build

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM node:20-alpine@sha256:658d0f63e501824d6c23e06d4bb95c71e7d704537c9d9272f488ac03a370d448 AS runtime

# Create a non-root user to prevent container-escape privilege escalation
RUN addgroup -S ttsgroup && adduser -S ttsuser -G ttsgroup

WORKDIR /app

# Copy dependency manifests and install production dependencies only
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force

# Copy compiled output from the builder stage
COPY --from=builder /build/dist ./dist

# Set ownership to the non-root user before switching
RUN chown -R ttsuser:ttsgroup /app

USER ttsuser

EXPOSE 3000

# HEALTHCHECK calls /health/live — a fast, process-level liveness probe that
# does not depend on external TTS providers and therefore never produces false
# negatives due to upstream unavailability.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD node -e "\
    require('http').get('http://localhost:3000/health/live', (r) => { \
      process.exit(r.statusCode === 200 ? 0 : 1); \
    }).on('error', () => process.exit(1));"

CMD ["node", "dist/server.js"]
