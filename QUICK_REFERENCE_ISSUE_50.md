# Quick Reference - Content Management API

## 🚀 Quick Start

```bash
cd api
npm install
cp .env.example .env
docker-compose up db -d
npm run dev
```

## 🔐 Authentication

```bash
# Login
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@predictiq.com","password":"admin123"}'

# Save token
TOKEN="your-jwt-token"
```

## 📡 API Endpoints

### Public

```bash
# Get content
GET /api/v1/content/:section

curl http://localhost:3000/api/v1/content/hero
```

### Admin (Requires Auth)

```bash
# Update content
POST /api/v1/content/:section
Authorization: Bearer <token>

curl -X POST http://localhost:3000/api/v1/content/hero \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"headline":"New","subheadline":"Text","ctaPrimary":"Go","ctaSecondary":"Learn"}'

# Get versions
GET /api/v1/content/:section/versions

curl http://localhost:3000/api/v1/content/hero/versions \
  -H "Authorization: Bearer $TOKEN"

# Get specific version
GET /api/v1/content/:section/versions/:version

curl http://localhost:3000/api/v1/content/hero/versions/1 \
  -H "Authorization: Bearer $TOKEN"

# Preview content
POST /api/v1/content/:section/preview

curl -X POST http://localhost:3000/api/v1/content/hero/preview \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"headline":"**Bold** text"}'
```

## 📋 Content Sections

### Hero
```json
{
  "headline": "string",
  "subheadline": "string",
  "ctaPrimary": "string",
  "ctaSecondary": "string"
}
```

### Features
```json
{
  "items": [{"title": "string", "description": "string"}]
}
```

### FAQ
```json
{
  "items": [{"question": "string", "answer": "string"}]
}
```

### Testimonials
```json
{
  "items": [{"name": "string", "role": "string", "content": "string"}]
}
```

### Announcements
```json
{
  "message": "string",
  "type": "info|warning|success"
}
```

## 🔧 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `JWT_SECRET` | your-secret-key | JWT signing secret |
| `JWT_EXPIRES_IN` | 24h | Token expiration |
| `CACHE_TTL` | 300 | Cache TTL (seconds) |

## 🎯 Key Features

- ✅ Version control
- ✅ JWT authentication
- ✅ Content validation
- ✅ Caching (5min)
- ✅ Markdown support
- ✅ Audit logging
- ✅ Preview mode

## 🔒 Default Credentials

**Email:** admin@predictiq.com  
**Password:** admin123

⚠️ Change in production!

## 📊 Database Tables

- `users` - Admin users
- `content` - Content versions
- `content_audit_log` - Change history

## 🧪 Testing

```bash
# Full test flow
TOKEN=$(curl -s -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@predictiq.com","password":"admin123"}' \
  | jq -r '.token')

curl http://localhost:3000/api/v1/content/hero

curl -X POST http://localhost:3000/api/v1/content/hero \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"headline":"Test","subheadline":"Update","ctaPrimary":"Go","ctaSecondary":"Learn"}'

curl http://localhost:3000/api/v1/content/hero/versions \
  -H "Authorization: Bearer $TOKEN"
```
