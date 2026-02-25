/**
 * Swagger UI Server for PredictIQ API Documentation
 * 
 * This server hosts the interactive Swagger UI for the PredictIQ API.
 * 
 * Usage:
 *   npm install
 *   npm start
 * 
 * Then visit: http://localhost:8080/api/docs
 */

const express = require('express');
const swaggerUi = require('swagger-ui-express');
const YAML = require('js-yaml');
const fs = require('fs');
const path = require('path');

const app = express();
const port = process.env.PORT || 8080;

// Load OpenAPI specification
const openApiPath = path.join(__dirname, '..', 'openapi.yaml');
const openApiSpec = YAML.load(fs.readFileSync(openApiPath, 'utf8'));

// Swagger UI options
const swaggerOptions = {
  customCss: `
    .swagger-ui .topbar { 
      background-color: #1a1a2e; 
    }
    .swagger-ui .topbar-wrapper img {
      content: url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjMwIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjx0ZXh0IHg9IjEwIiB5PSIyMCIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjIwIiBmaWxsPSJ3aGl0ZSI+UHJlZGljdElRPC90ZXh0Pjwvc3ZnPg==');
    }
    .swagger-ui .info .title {
      color: #1a1a2e;
    }
  `,
  customSiteTitle: 'PredictIQ API Documentation',
  customfavIcon: '/favicon.ico',
  swaggerOptions: {
    persistAuthorization: true,
    displayRequestDuration: true,
    filter: true,
    syntaxHighlight: {
      activate: true,
      theme: 'monokai'
    },
    tryItOutEnabled: true,
    requestSnippetsEnabled: true,
    requestSnippets: {
      generators: {
        curl_bash: {
          title: 'cURL (bash)',
          syntax: 'bash'
        },
        curl_powershell: {
          title: 'cURL (PowerShell)',
          syntax: 'powershell'
        },
        curl_cmd: {
          title: 'cURL (CMD)',
          syntax: 'bash'
        }
      },
      defaultExpanded: true,
      languages: null
    }
  }
};

// Enable CORS for development
app.use((req, res, next) => {
  res.header('Access-Control-Allow-Origin', '*');
  res.header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.header('Access-Control-Allow-Headers', 'Content-Type, Authorization');
  
  if (req.method === 'OPTIONS') {
    return res.sendStatus(200);
  }
  
  next();
});

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({
    status: 'healthy',
    timestamp: new Date().toISOString(),
    version: openApiSpec.info.version
  });
});

// Serve OpenAPI spec as JSON
app.get('/openapi.json', (req, res) => {
  res.json(openApiSpec);
});

// Serve OpenAPI spec as YAML
app.get('/openapi.yaml', (req, res) => {
  res.type('text/yaml');
  res.send(fs.readFileSync(openApiPath, 'utf8'));
});

// Redirect root to API docs
app.get('/', (req, res) => {
  res.redirect('/api/docs');
});

// Serve Swagger UI
app.use('/api/docs', swaggerUi.serve, swaggerUi.setup(openApiSpec, swaggerOptions));

// 404 handler
app.use((req, res) => {
  res.status(404).json({
    error: 'Not Found',
    message: `Route ${req.url} not found`,
    availableRoutes: [
      '/api/docs - Interactive API documentation',
      '/openapi.json - OpenAPI specification (JSON)',
      '/openapi.yaml - OpenAPI specification (YAML)',
      '/health - Health check endpoint'
    ]
  });
});

// Error handler
app.use((err, req, res, next) => {
  console.error('Error:', err);
  res.status(500).json({
    error: 'Internal Server Error',
    message: err.message
  });
});

// Start server
app.listen(port, () => {
  console.log('╔════════════════════════════════════════════════════════════╗');
  console.log('║                                                            ║');
  console.log('║           PredictIQ API Documentation Server              ║');
  console.log('║                                                            ║');
  console.log('╚════════════════════════════════════════════════════════════╝');
  console.log('');
  console.log(`🚀 Server running on port ${port}`);
  console.log('');
  console.log('📚 Available endpoints:');
  console.log(`   • Swagger UI:    http://localhost:${port}/api/docs`);
  console.log(`   • OpenAPI JSON:  http://localhost:${port}/openapi.json`);
  console.log(`   • OpenAPI YAML:  http://localhost:${port}/openapi.yaml`);
  console.log(`   • Health Check:  http://localhost:${port}/health`);
  console.log('');
  console.log('Press Ctrl+C to stop the server');
  console.log('');
});

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('\n🛑 Shutting down gracefully...');
  process.exit(0);
});

process.on('SIGINT', () => {
  console.log('\n🛑 Shutting down gracefully...');
  process.exit(0);
});
