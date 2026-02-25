# PredictIQ Swagger UI Server

Interactive API documentation server for the PredictIQ prediction market platform.

## Quick Start

```bash
# Install dependencies
npm install

# Start server
npm start

# Visit http://localhost:8080/api/docs
```

## Features

- 🎨 Interactive Swagger UI
- 📝 OpenAPI 3.0 specification
- 🔄 Auto-reload on spec changes
- 🌐 CORS enabled for development
- 💾 Export spec as JSON/YAML
- 🏥 Health check endpoint

## Available Endpoints

- **Swagger UI**: `http://localhost:8080/api/docs`
- **OpenAPI JSON**: `http://localhost:8080/openapi.json`
- **OpenAPI YAML**: `http://localhost:8080/openapi.yaml`
- **Health Check**: `http://localhost:8080/health`

## Development

```bash
# Install dependencies
npm install

# Start with auto-reload
npm run dev

# The server will restart automatically when files change
```

## Configuration

### Port

Set custom port using environment variable:

```bash
PORT=3000 npm start
```

### Custom OpenAPI Spec

Edit the path in `server.js`:

```javascript
const openApiPath = path.join(__dirname, '..', 'openapi.yaml');
```

## Docker

```bash
# Build image
docker build -t predictiq-swagger .

# Run container
docker run -p 8080:8080 predictiq-swagger

# Visit http://localhost:8080/api/docs
```

## Deployment

### Deploy to Vercel

```bash
npm install -g vercel
vercel
```

### Deploy to Heroku

```bash
heroku create predictiq-api-docs
git push heroku main
```

### Deploy to Railway

```bash
railway login
railway init
railway up
```

## Customization

### Theme

Edit `swaggerOptions.customCss` in `server.js`:

```javascript
customCss: `
  .swagger-ui .topbar { 
    background-color: #your-color; 
  }
`
```

### Logo

Replace the base64 SVG in `customCss`:

```javascript
.swagger-ui .topbar-wrapper img {
  content: url('your-logo-url');
}
```

## Troubleshooting

### Port Already in Use

```bash
# Use different port
PORT=3001 npm start
```

### CORS Issues

CORS is enabled by default. To restrict origins:

```javascript
app.use((req, res, next) => {
  res.header('Access-Control-Allow-Origin', 'https://your-domain.com');
  // ...
});
```

### Spec Not Loading

1. Check `openapi.yaml` path is correct
2. Validate YAML syntax
3. Check console for errors

## License

MIT
