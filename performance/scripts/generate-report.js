#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const REPORTS_DIR = path.join(__dirname, '..', 'backend', 'reports');
const OUTPUT_FILE = path.join(REPORTS_DIR, 'performance-report.html');

function loadTestResults() {
  const results = {};
  const files = [
    'smoke-test-summary.json',
    'load-test-summary.json',
    'stress-test-summary.json',
    'spike-test-summary.json',
    'rate-limit-test-summary.json',
    'cache-test-summary.json',
  ];

  files.forEach(file => {
    const filePath = path.join(REPORTS_DIR, file);
    if (fs.existsSync(filePath)) {
      const testName = file.replace('-summary.json', '');
      results[testName] = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    }
  });

  return results;
}

function generateHTML(results) {
  const timestamp = new Date().toISOString();
  
  let html = `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Performance Test Report - PredictIQ</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { 
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
      background: #f5f5f5;
      padding: 20px;
    }
    .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
    h1 { color: #333; margin-bottom: 10px; }
    .timestamp { color: #666; font-size: 14px; margin-bottom: 30px; }
    .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 40px; }
    .metric-card { background: #f8f9fa; padding: 20px; border-radius: 6px; border-left: 4px solid #4CAF50; }
    .metric-card.warning { border-left-color: #ff9800; }
    .metric-card.error { border-left-color: #f44336; }
    .metric-label { font-size: 12px; color: #666; text-transform: uppercase; margin-bottom: 5px; }
    .metric-value { font-size: 28px; font-weight: bold; color: #333; }
    .metric-unit { font-size: 14px; color: #666; }
    table { width: 100%; border-collapse: collapse; margin: 20px 0; }
    th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
    th { background: #4CAF50; color: white; font-weight: 600; }
    tr:hover { background: #f5f5f5; }
    .pass { color: #4CAF50; font-weight: bold; }
    .fail { color: #f44336; font-weight: bold; }
    .test-section { margin: 40px 0; }
    .test-section h2 { color: #333; margin-bottom: 20px; padding-bottom: 10px; border-bottom: 2px solid #4CAF50; }
    .status-badge { 
      display: inline-block; 
      padding: 4px 12px; 
      border-radius: 12px; 
      font-size: 12px; 
      font-weight: bold;
    }
    .status-badge.pass { background: #e8f5e9; color: #2e7d32; }
    .status-badge.fail { background: #ffebee; color: #c62828; }
  </style>
</head>
<body>
  <div class="container">
    <h1>🚀 Performance Test Report</h1>
    <div class="timestamp">Generated: ${timestamp}</div>
    
    <div class="summary">
      ${generateSummaryCards(results)}
    </div>
    
    ${generateTestSections(results)}
    
    <div class="test-section">
      <h2>Performance Targets</h2>
      <table>
        <thead>
          <tr>
            <th>Metric</th>
            <th>Target</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>API Response Time (p95)</td>
            <td>&lt; 200ms</td>
            <td>${checkTarget(results, 'p95', 200)}</td>
          </tr>
          <tr>
            <td>API Response Time (p99)</td>
            <td>&lt; 500ms</td>
            <td>${checkTarget(results, 'p99', 500)}</td>
          </tr>
          <tr>
            <td>Error Rate</td>
            <td>&lt; 0.1%</td>
            <td>${checkErrorRate(results)}</td>
          </tr>
          <tr>
            <td>Cache Hit Rate</td>
            <td>&gt; 80%</td>
            <td>${checkCacheHitRate(results)}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</body>
</html>
  `;
  
  return html;
}

function generateSummaryCards(results) {
  const cards = [];
  
  // Total requests
  let totalRequests = 0;
  Object.values(results).forEach(result => {
    if (result.metrics && result.metrics.http_reqs) {
      totalRequests += result.metrics.http_reqs.values.count || 0;
    }
  });
  
  cards.push(`
    <div class="metric-card">
      <div class="metric-label">Total Requests</div>
      <div class="metric-value">${totalRequests.toLocaleString()}</div>
    </div>
  `);
  
  // Average response time
  const loadTest = results['load-test'];
  if (loadTest && loadTest.metrics && loadTest.metrics.http_req_duration) {
    const avgTime = loadTest.metrics.http_req_duration.values.avg.toFixed(2);
    const cardClass = avgTime < 200 ? '' : 'warning';
    cards.push(`
      <div class="metric-card ${cardClass}">
        <div class="metric-label">Avg Response Time</div>
        <div class="metric-value">${avgTime}<span class="metric-unit">ms</span></div>
      </div>
    `);
  }
  
  // P95 response time
  if (loadTest && loadTest.metrics && loadTest.metrics.http_req_duration) {
    const p95 = loadTest.metrics.http_req_duration.values['p(95)'].toFixed(2);
    const cardClass = p95 < 200 ? '' : p95 < 300 ? 'warning' : 'error';
    cards.push(`
      <div class="metric-card ${cardClass}">
        <div class="metric-label">P95 Response Time</div>
        <div class="metric-value">${p95}<span class="metric-unit">ms</span></div>
      </div>
    `);
  }
  
  // Error rate
  if (loadTest && loadTest.metrics && loadTest.metrics.http_req_failed) {
    const errorRate = (loadTest.metrics.http_req_failed.values.rate * 100).toFixed(2);
    const cardClass = errorRate < 0.1 ? '' : errorRate < 1 ? 'warning' : 'error';
    cards.push(`
      <div class="metric-card ${cardClass}">
        <div class="metric-label">Error Rate</div>
        <div class="metric-value">${errorRate}<span class="metric-unit">%</span></div>
      </div>
    `);
  }
  
  return cards.join('');
}

function generateTestSections(results) {
  let sections = '';
  
  Object.entries(results).forEach(([testName, result]) => {
    sections += `
      <div class="test-section">
        <h2>${formatTestName(testName)}</h2>
        <table>
          <thead>
            <tr>
              <th>Metric</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            ${generateMetricRows(result)}
          </tbody>
        </table>
      </div>
    `;
  });
  
  return sections;
}

function generateMetricRows(result) {
  if (!result.metrics) return '<tr><td colspan="2">No metrics available</td></tr>';
  
  const rows = [];
  const metrics = result.metrics;
  
  if (metrics.http_reqs) {
    rows.push(`<tr><td>Total Requests</td><td>${metrics.http_reqs.values.count}</td></tr>`);
    rows.push(`<tr><td>Requests/sec</td><td>${metrics.http_reqs.values.rate.toFixed(2)}</td></tr>`);
  }
  
  if (metrics.http_req_duration) {
    rows.push(`<tr><td>Avg Response Time</td><td>${metrics.http_req_duration.values.avg.toFixed(2)}ms</td></tr>`);
    rows.push(`<tr><td>Min Response Time</td><td>${metrics.http_req_duration.values.min.toFixed(2)}ms</td></tr>`);
    rows.push(`<tr><td>Max Response Time</td><td>${metrics.http_req_duration.values.max.toFixed(2)}ms</td></tr>`);
    rows.push(`<tr><td>P95 Response Time</td><td>${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms</td></tr>`);
    rows.push(`<tr><td>P99 Response Time</td><td>${metrics.http_req_duration.values['p(99)'].toFixed(2)}ms</td></tr>`);
  }
  
  if (metrics.http_req_failed) {
    const errorRate = (metrics.http_req_failed.values.rate * 100).toFixed(2);
    rows.push(`<tr><td>Error Rate</td><td>${errorRate}%</td></tr>`);
  }
  
  return rows.join('');
}

function formatTestName(name) {
  return name.split('-').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
}

function checkTarget(results, metric, threshold) {
  const loadTest = results['load-test'];
  if (!loadTest || !loadTest.metrics || !loadTest.metrics.http_req_duration) {
    return '<span class="status-badge">N/A</span>';
  }
  
  const value = loadTest.metrics.http_req_duration.values[`p(${metric.slice(1)})`];
  const pass = value < threshold;
  return `<span class="status-badge ${pass ? 'pass' : 'fail'}">${pass ? 'PASS' : 'FAIL'}</span>`;
}

function checkErrorRate(results) {
  const loadTest = results['load-test'];
  if (!loadTest || !loadTest.metrics || !loadTest.metrics.http_req_failed) {
    return '<span class="status-badge">N/A</span>';
  }
  
  const rate = loadTest.metrics.http_req_failed.values.rate * 100;
  const pass = rate < 0.1;
  return `<span class="status-badge ${pass ? 'pass' : 'fail'}">${pass ? 'PASS' : 'FAIL'}</span>`;
}

function checkCacheHitRate(results) {
  const cacheTest = results['cache-test'];
  if (!cacheTest || !cacheTest.metrics || !cacheTest.metrics.cache_hit_rate) {
    return '<span class="status-badge">N/A</span>';
  }
  
  const rate = cacheTest.metrics.cache_hit_rate.values.rate * 100;
  const pass = rate > 80;
  return `<span class="status-badge ${pass ? 'pass' : 'fail'}">${pass ? 'PASS' : 'FAIL'}</span>`;
}

// Main execution
console.log('📊 Generating performance report...');

const results = loadTestResults();
const html = generateHTML(results);

fs.writeFileSync(OUTPUT_FILE, html);

console.log(`✓ Report generated: ${OUTPUT_FILE}`);
console.log(`\nOpen in browser: file://${OUTPUT_FILE}`);
