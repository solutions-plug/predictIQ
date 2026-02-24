# Quick Reference - Landing Page API Service

## 🚀 Quick Start

```bash
cd api
npm install
cp .env.example .env
docker-compose up db -d
npm run dev
```

## 📡 API Endpoints

### Health Check
```bash
GET /health
```

### Newsletter Signup
```bash
POST /api/v1/newsletter
Content-Type: application/json

{"email": "user@example.com"}
```

### Analytics
```bash
GET /api/v1/analytics
```

## 🔧 Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start development server |
| `npm run build` | Build TypeScript |
| `npm start` | Start production server |
| `./verify.sh` | Run verification tests |
| `docker-compose up` | Start with Docker |

## 🌍 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NODE_ENV` | development | Environment mode |
| `PORT` | 3000 | Server port |
| `DATABASE_URL` | postgresql://... | PostgreSQL connection |
| `CORS_ORIGIN` | http://localhost:3001 | Allowed origin |
| `RATE_LIMIT_MAX_REQUESTS` | 100 | Max requests per window |

## 🐳 Docker

```bash
# Local development
docker-compose up -d

# Build image
docker build -t predictiq-api .

# Run container
docker run -p 3000:3000 --env-file .env predictiq-api
```

## ☸️ Kubernetes

```bash
# Deploy
kubectl apply -f k8s-deployment.yaml

# Check status
kubectl get pods -l app=predictiq-api

# View logs
kubectl logs -l app=predictiq-api
```

## 🧪 Testing

```bash
# Health check
curl http://localhost:3000/health

# Newsletter signup
curl -X POST http://localhost:3000/api/v1/newsletter \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}'

# Analytics
curl http://localhost:3000/api/v1/analytics
```

## 📊 Database Schema

```sql
CREATE TABLE newsletter_signups (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## 🔒 Security Features

- ✅ Rate limiting (100 req/15min)
- ✅ CORS protection
- ✅ Input validation
- ✅ SQL injection prevention
- ✅ Error sanitization

## 📝 Logging

Logs include:
- HTTP requests/responses
- Database queries
- Errors and warnings
- Performance metrics

## 🎯 Key Features

- TypeScript for type safety
- PostgreSQL with connection pooling
- API versioning (/api/v1)
- Structured logging (Pino)
- Error handling middleware
- Rate limiting
- Docker & Kubernetes ready

## 🔗 Useful Links

- [Full Documentation](./api/README.md)
- [PR Summary](./PR_SUMMARY_ISSUE_44.md)
- [Project README](./README.md)
