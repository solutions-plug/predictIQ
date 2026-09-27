#!/usr/bin/env node

/**
 * OpenAPI to Markdown Generator
 * 
 * Generates API_SPEC.md from services/api/openapi.yaml
 * Ensures single source of truth for API documentation.
 * 
 * Usage:
 *   node generate-api-spec.js [--check] [--output <path>]
 * 
 * Options:
 *   --check    Verify API_SPEC.md is in sync with openapi.yaml (exit 1 if not)
 *   --output   Output file path (default: API_SPEC.md)
 */

const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');

const args = process.argv.slice(2);
const checkMode = args.includes('--check');
const outputIdx = args.indexOf('--output');
const outputPath = outputIdx !== -1 ? args[outputIdx + 1] : path.join(__dirname, '../API_SPEC.md');
const openApiPath = path.join(__dirname, '../services/api/openapi.yaml');

// Base URL shown in the generated docs. Sourced from config/env so it reflects
// the real deployment bind address; defaults to the services/api default bind
// address (0.0.0.0:8080).
const baseUrl = process.env.API_BASE_URL || `http://0.0.0.0:${process.env.PORT || 8080}`;

// HTTP methods defined by the OpenAPI 3.x specification for path items.
const HTTP_METHODS = ['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace'];

/**
 * Load and parse OpenAPI spec using a real YAML parser.
 */
function loadOpenApiSpec() {
  try {
    const content = fs.readFileSync(openApiPath, 'utf8');
    const doc = yaml.load(content);
    if (!doc || typeof doc !== 'object') {
      throw new Error('OpenAPI document did not parse into an object');
    }
    const info = doc.info || {};
    return {
      doc,
      title: info.title || '',
      version: info.version || '',
      description: typeof info.description === 'string' ? info.description.trim() : '',
    };
  } catch (e) {
    console.error(`Failed to load OpenAPI spec: ${e.message}`);
    process.exit(1);
  }
}

/**
 * Extract endpoints by iterating the parsed `paths` object directly.
 * Covers every HTTP method present in the document (including head/options).
 */
function extractEndpoints(doc) {
  const endpoints = [];
  const paths = (doc && doc.paths) || {};

  for (const [routePath, pathItem] of Object.entries(paths)) {
    if (!pathItem || typeof pathItem !== 'object') continue;
    for (const method of HTTP_METHODS) {
      if (pathItem[method]) {
        endpoints.push({
          path: routePath,
          method: method.toUpperCase(),
        });
      }
    }
  }

  return endpoints;
}

/**
 * Generate markdown from OpenAPI spec
 */
function generateMarkdown(spec) {
  const endpoints = extractEndpoints(spec.doc);
  
  let md = `# ${spec.title} - API Specification

**Version:** ${spec.version}

${spec.description}

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Endpoints](#endpoints)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)

## Overview

### Base URL

\`\`\`
${baseUrl}
\`\`\`

### API Versioning

The API uses URL path versioning (\`/api/v1/\`). The current stable version is **v1**.

Clients may also send an \`API-Version\` header (e.g. \`API-Version: v1\`) to explicitly
declare the version they target. If omitted, the server defaults to the current version.

### Deprecation Policy

When a version is deprecated:
- Responses will include a \`Deprecation\` header set to \`true\`.
- A \`Sunset\` header will indicate the date after which the version will be removed.
- A \`Link\` header will point to migration documentation.

Clients should monitor these headers and migrate before the sunset date.

Deprecated versions are supported for a minimum of **12 months** after the deprecation
announcement before being removed.

## Authentication

The API uses Bearer token authentication. Include your API key in the \`Authorization\` header:

\`\`\`
Authorization: Bearer YOUR_API_KEY
\`\`\`

## Endpoints

`;

  // Group endpoints by category
  const grouped = {};
  endpoints.forEach(ep => {
    const category = ep.path.split('/')[1] || 'general';
    if (!grouped[category]) grouped[category] = [];
    grouped[category].push(ep);
  });

  Object.entries(grouped).forEach(([category, eps]) => {
    md += `### ${category.charAt(0).toUpperCase() + category.slice(1)}\n\n`;
    eps.forEach(ep => {
      md += `#### ${ep.method} ${ep.path}\n\n`;
      md += `\`\`\`\n${ep.method} ${ep.path}\n\`\`\`\n\n`;
    });
  });

  // Error Handling
  md += `## Error Handling

All errors are returned as JSON with the following structure:

\`\`\`json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {}
  }
}
\`\`\`

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| INVALID_REQUEST | 400 | Request validation failed |
| UNAUTHORIZED | 401 | Authentication required or failed |
| FORBIDDEN | 403 | Insufficient permissions |
| NOT_FOUND | 404 | Resource not found |
| CONFLICT | 409 | Resource conflict (e.g., duplicate) |
| RATE_LIMITED | 429 | Rate limit exceeded |
| INTERNAL_ERROR | 500 | Internal server error |

## Rate Limiting

The API implements rate limiting to ensure fair usage:

- **Rate Limit:** 1000 requests per minute per API key
- **Headers:**
  - \`X-RateLimit-Limit\`: Maximum requests per window
  - \`X-RateLimit-Remaining\`: Requests remaining in current window
  - \`X-RateLimit-Reset\`: Unix timestamp when limit resets

When rate limited (HTTP 429), the response includes a \`Retry-After\` header indicating
how many seconds to wait before retrying.

---

**Generated from:** \`services/api/openapi.yaml\`  
**Last Updated:** ${new Date().toISOString()}  
**Note:** This file is auto-generated. Do not edit directly. Update \`services/api/openapi.yaml\` instead.
`;

  return md;
}

/**
 * Strip the volatile generation timestamp line so content comparisons in
 * --check mode are not defeated by a value that changes on every run.
 */
function normalizeForComparison(markdown) {
  return markdown.replace(/^\*\*Last Updated:\*\*.*$/m, '**Last Updated:** <normalized>');
}

/**
 * Main execution
 */
function main() {
  console.log('📄 Generating API specification from OpenAPI...\n');
  
  const spec = loadOpenApiSpec();
  const markdown = generateMarkdown(spec);
  
  if (checkMode) {
    // Check if current file matches generated content, ignoring the
    // generation timestamp which differs on every invocation.
    if (fs.existsSync(outputPath)) {
      const current = fs.readFileSync(outputPath, 'utf8');
      if (normalizeForComparison(current) === normalizeForComparison(markdown)) {
        console.log('✅ API_SPEC.md is in sync with openapi.yaml');
        process.exit(0);
      } else {
        console.error('❌ API_SPEC.md is out of sync with openapi.yaml');
        console.error('\nRun the following to update:');
        console.error('  node scripts/generate-api-spec.js');
        process.exit(1);
      }
    } else {
      console.error('❌ API_SPEC.md not found');
      process.exit(1);
    }
  } else {
    // Generate and write file
    fs.writeFileSync(outputPath, markdown);
    console.log(`✅ Generated: ${outputPath}`);
    console.log(`\n📝 Next steps:`);
    console.log('   1. Review changes: git diff API_SPEC.md');
    console.log('   2. Commit: git add API_SPEC.md && git commit -m "chore: regenerate API_SPEC.md"');
  }
}

main();
