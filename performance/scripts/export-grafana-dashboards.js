#!/usr/bin/env node

/**
 * Grafana Dashboard Export Script
 * 
 * Exports dashboards from a running Grafana instance and saves them to the repo.
 * This ensures dashboard changes made in the UI are persisted to version control.
 * 
 * Usage:
 *   node export-grafana-dashboards.js [--url <grafana-url>] [--api-key <key>]
 * 
 * Environment Variables:
 *   GRAFANA_URL - Grafana instance URL (default: http://localhost:3000)
 *   GRAFANA_API_KEY - Grafana API key with admin permissions
 * 
 * Example:
 *   GRAFANA_API_KEY=abc123 node export-grafana-dashboards.js
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const http = require('http');

// Parse CLI arguments
const args = process.argv.slice(2);
const urlIdx = args.indexOf('--url');
const keyIdx = args.indexOf('--api-key');

const GRAFANA_URL = urlIdx !== -1 ? args[urlIdx + 1] : process.env.GRAFANA_URL || 'http://localhost:3000';
const GRAFANA_API_KEY = keyIdx !== -1 ? args[keyIdx + 1] : process.env.GRAFANA_API_KEY;
const CONFIG_DIR = path.join(__dirname, '..');
const DASHBOARDS_TO_EXPORT = [
  'grafana-dashboard.json',
  'grafana-slo-dashboard.json',
];

if (!GRAFANA_API_KEY) {
  console.error('❌ Error: GRAFANA_API_KEY environment variable or --api-key flag is required');
  console.error('   Set GRAFANA_API_KEY=<your-api-key> or pass --api-key <key>');
  process.exit(1);
}

/**
 * Make HTTP request to Grafana API
 */
function makeRequest(method, path, body = null) {
  return new Promise((resolve, reject) => {
    const url = new URL(GRAFANA_URL);
    const isHttps = url.protocol === 'https:';
    const client = isHttps ? https : http;

    const options = {
      hostname: url.hostname,
      port: url.port,
      path: path,
      method: method,
      headers: {
        'Authorization': `Bearer ${GRAFANA_API_KEY}`,
        'Content-Type': 'application/json',
      },
    };

    const req = client.request(options, (res) => {
      let data = '';

      res.on('data', (chunk) => {
        data += chunk;
      });

      res.on('end', () => {
        if (res.statusCode >= 200 && res.statusCode < 300) {
          try {
            resolve(JSON.parse(data));
          } catch (e) {
            resolve(data);
          }
        } else {
          reject(new Error(`HTTP ${res.statusCode}: ${data}`));
        }
      });
    });

    req.on('error', reject);

    if (body) {
      req.write(JSON.stringify(body));
    }

    req.end();
  });
}

/**
 * Get dashboard by UID
 */
async function getDashboard(uid) {
  try {
    const response = await makeRequest('GET', `/api/dashboards/uid/${uid}`);
    return response.dashboard;
  } catch (e) {
    console.error(`Failed to fetch dashboard ${uid}:`, e.message);
    return null;
  }
}

/**
 * Search for dashboards by tag
 */
async function searchDashboards(tag) {
  try {
    const response = await makeRequest('GET', `/api/search?tag=${tag}`);
    return response;
  } catch (e) {
    console.error(`Failed to search dashboards:`, e.message);
    return [];
  }
}

/**
 * Export dashboard to file
 */
function exportDashboard(dashboard, filename) {
  const filepath = path.join(CONFIG_DIR, filename);
  
  // Remove internal Grafana fields
  const cleanDashboard = {
    ...dashboard,
    id: null,
    uid: null,
    version: 0,
  };

  fs.writeFileSync(filepath, JSON.stringify(cleanDashboard, null, 2));
  console.log(`✓ Exported: ${filename}`);
  return filepath;
}

/**
 * Main export logic
 */
async function main() {
  console.log(`📊 Grafana Dashboard Export`);
  console.log(`Grafana URL: ${GRAFANA_URL}`);
  console.log('─'.repeat(60));

  try {
    // Test connection
    await makeRequest('GET', '/api/health');
    console.log('✓ Connected to Grafana\n');
  } catch (e) {
    console.error('❌ Failed to connect to Grafana:', e.message);
    process.exit(1);
  }

  // Search for dashboards with 'performance' tag
  console.log('Searching for performance dashboards...');
  const dashboards = await searchDashboards('performance');

  if (dashboards.length === 0) {
    console.log('⚠️  No dashboards found with "performance" tag');
    console.log('   Create dashboards in Grafana and tag them with "performance"');
    process.exit(1);
  }

  console.log(`Found ${dashboards.length} dashboard(s)\n`);

  let exported = 0;

  // Export each dashboard
  for (const dashboard of dashboards) {
    console.log(`Exporting: ${dashboard.title}`);
    
    const fullDashboard = await getDashboard(dashboard.uid);
    if (fullDashboard) {
      // Map dashboard title to filename
      let filename;
      if (dashboard.title.toLowerCase().includes('slo')) {
        filename = 'grafana-slo-dashboard.json';
      } else {
        filename = 'grafana-dashboard.json';
      }

      exportDashboard(fullDashboard, filename);
      exported++;
    }
  }

  console.log(`\n✅ Successfully exported ${exported} dashboard(s)`);
  console.log('\n📝 Next steps:');
  console.log('   1. Review changes: git diff performance/config/grafana-*.json');
  console.log('   2. Commit changes: git add performance/config/grafana-*.json');
  console.log('   3. Push to repository: git push');
}

main().catch(err => {
  console.error('❌ Export failed:', err.message);
  process.exit(1);
});
