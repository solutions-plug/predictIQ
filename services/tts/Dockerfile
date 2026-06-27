# TTS Service Dockerfile
# Builds and runs the Text-to-Speech service with health checks

FROM node:20-alpine

WORKDIR /app

# Install dependencies
COPY package*.json ./
RUN npm ci --only=production

# Copy source code
COPY src ./src
COPY tsconfig.json ./

# Build TypeScript
RUN npm run build 2>/dev/null || npx tsc

EXPOSE 3000

# Health check endpoint
# Checks Google Cloud TTS API connectivity and service health
# Returns 200 with {"status": "ok"} when healthy
# Returns 503 with error details when upstream is unavailable
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD node -e "require('http').get('http://localhost:3000/health', (r) => {if (r.statusCode !== 200) throw new Error(r.statusCode)})" || exit 1

CMD ["npm", "start"]
