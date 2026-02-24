# PredictIQ Landing Page API

Backend API service for PredictIQ landing page.

## Features

- ✅ Express.js with TypeScript
- ✅ PostgreSQL database connection
- ✅ API versioning (`/api/v1`)
- ✅ Pino logging
- ✅ CORS configuration
- ✅ Error handling middleware
- ✅ Rate limiting
- ✅ Health check endpoint
- ✅ Docker support

## Quick Start

### Local Development

1. **Install dependencies:**
```bash
npm install
```

2. **Set up environment:**
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. **Start PostgreSQL:**
```bash
docker-compose up db -d
```

4. **Run development server:**
```bash
npm run dev
```

Server runs on `http://localhost:3000`

### Docker Deployment

```bash
docker-compose up -d
```

## API Endpoints

### Health Check
```
GET /health
```

Response:
```json
{
  "status": "ok",
  "timestamp": "2026-02-24T07:34:17.144Z",
  "database": "connected"
}
```

### Newsletter Signup
```
POST /api/v1/newsletter
Content-Type: application/json

{
  "email": "user@example.com"
}
```

### Analytics
```
GET /api/v1/analytics
```

Response:
```json
{
  "status": "success",
  "data": {
    "totalSignups": 42
  }
}
```

## Configuration

Environment variables (see `.env.example`):

- `NODE_ENV` - Environment (development/production)
- `PORT` - Server port (default: 3000)
- `DATABASE_URL` - PostgreSQL connection string
- `CORS_ORIGIN` - Allowed CORS origin
- `RATE_LIMIT_WINDOW_MS` - Rate limit window (default: 15 min)
- `RATE_LIMIT_MAX_REQUESTS` - Max requests per window (default: 100)

## Scripts

- `npm run dev` - Start development server with hot reload
- `npm run build` - Build TypeScript to JavaScript
- `npm start` - Start production server
- `npm run lint` - Run ESLint

## Project Structure

```
api/
├── src/
│   ├── config/          # Configuration files
│   │   ├── index.ts     # Main config
│   │   └── database.ts  # Database connection
│   ├── middleware/      # Express middleware
│   │   ├── errorHandler.ts
│   │   └── rateLimiter.ts
│   ├── routes/          # API routes
│   │   ├── health.ts
│   │   └── landing.ts
│   ├── utils/           # Utilities
│   │   └── logger.ts
│   └── index.ts         # Application entry point
├── Dockerfile
├── docker-compose.yml
├── init.sql             # Database schema
└── package.json
```

## Rate Limiting

Default: 100 requests per 15 minutes per IP address.

## Error Handling

All errors return JSON:
```json
{
  "status": "error",
  "message": "Error description"
}
```

## Logging

Uses Pino for structured logging. Logs include:
- HTTP requests/responses
- Database queries
- Errors and warnings

## Database Schema

### newsletter_signups
- `id` - Serial primary key
- `email` - Unique email address
- `created_at` - Timestamp

## License

MIT
