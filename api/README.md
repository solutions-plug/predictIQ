# PredictIQ CMS API

Content Management System API for PredictIQ landing page.

## Features

- ✅ Dynamic content management
- ✅ Version control for content changes
- ✅ JWT authentication for admin endpoints
- ✅ Content validation
- ✅ Caching for performance
- ✅ Markdown support
- ✅ Audit logging
- ✅ Content preview

## Quick Start

```bash
npm install
cp .env.example .env
docker-compose up db -d
npm run dev
```

## API Endpoints

### Public Endpoints

#### Get Content
```
GET /api/v1/content/:section
```

Example:
```bash
curl http://localhost:3000/api/v1/content/hero
```

Response:
```json
{
  "section": "hero",
  "headline": "Welcome to PredictIQ",
  "subheadline": "Decentralized prediction markets",
  "ctaPrimary": "Get Started",
  "ctaSecondary": "Learn More",
  "version": 1,
  "lastUpdated": "2026-02-24T07:54:36.334Z"
}
```

### Admin Endpoints (Requires Authentication)

#### Login
```
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "admin@predictiq.com",
  "password": "admin123"
}
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": 1,
    "email": "admin@predictiq.com"
  }
}
```

#### Update Content
```
POST /api/v1/content/:section
Authorization: Bearer <token>
Content-Type: application/json

{
  "headline": "New Headline",
  "subheadline": "New Subheadline",
  "ctaPrimary": "Start Now",
  "ctaSecondary": "Explore"
}
```

#### Get Version History
```
GET /api/v1/content/:section/versions?limit=10
Authorization: Bearer <token>
```

#### Get Specific Version
```
GET /api/v1/content/:section/versions/:version
Authorization: Bearer <token>
```

#### Preview Content
```
POST /api/v1/content/:section/preview
Authorization: Bearer <token>
Content-Type: application/json

{
  "headline": "**Bold** headline with markdown"
}
```

## Content Sections

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
  "items": [
    {
      "title": "string",
      "description": "string"
    }
  ]
}
```

### FAQ
```json
{
  "items": [
    {
      "question": "string",
      "answer": "string"
    }
  ]
}
```

### Testimonials
```json
{
  "items": [
    {
      "name": "string",
      "role": "string",
      "content": "string",
      "avatar": "string"
    }
  ]
}
```

### Announcements
```json
{
  "message": "string",
  "type": "info|warning|success"
}
```

## Environment Variables

```env
NODE_ENV=development
PORT=3000
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/predictiq
JWT_SECRET=your-secret-key
JWT_EXPIRES_IN=24h
CACHE_TTL=300
CORS_ORIGIN=http://localhost:3001
```

## Testing

```bash
# Login
TOKEN=$(curl -s -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@predictiq.com","password":"admin123"}' \
  | jq -r '.token')

# Get content
curl http://localhost:3000/api/v1/content/hero

# Update content
curl -X POST http://localhost:3000/api/v1/content/hero \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"headline":"Updated","subheadline":"New text","ctaPrimary":"Go","ctaSecondary":"Learn"}'

# Get versions
curl http://localhost:3000/api/v1/content/hero/versions \
  -H "Authorization: Bearer $TOKEN"
```

## Features

### Version Control
- Every content update creates a new version
- Old versions are preserved
- Version history accessible via API

### Caching
- Content cached for 5 minutes (configurable)
- Cache invalidated on updates
- Improves read performance

### Validation
- Content validated against section schemas
- Required fields enforced
- Prevents invalid content

### Audit Log
- All changes logged with user and timestamp
- Audit trail for compliance

### Markdown Support
- Markdown automatically rendered in preview
- Supports **bold**, *italic*, links, etc.

## Database Schema

```sql
users (id, email, password_hash, is_admin)
content (id, section, content, version, created_by, is_active)
content_audit_log (id, section, version, action, user_id)
```

## Security

- JWT authentication for admin endpoints
- Bcrypt password hashing
- Rate limiting
- CORS protection
- Input validation

## Performance

- In-memory caching (node-cache)
- Database indexes on frequently queried fields
- Efficient version queries

## License

MIT
