#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const REPORTS_DIR = path.join(__dirname, '..', 'backend', 'reports');

function loadResults(filename) {
  const filePath = path.join(REPORTS_DIR, filename);
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function compareMetrics(baseline, current, metricPath) {
  const baselineValue = getNestedValue(baseline, metricPath);
  const currentValue = getNestedValue(current, metricPath);
  
  if (baselineValue === null || currentValue === null) {
    return null;
  }
  
  const change = ((currentValue - baselineValue) / baselineValue) * 100;
  return {
    baseline: baselineValue,
    current: currentValue,
    change: change,
    regression: change > 10,
  };
}

function getNestedValue(obj, path) {
  return path.split('.').reduce((current, key) => {
    return current && current[key] !== undefined ? current[key] : null;
  }, obj);
}

function formatChange(change) {
  const sign = change > 0 ? '+' : '';
  return `${sign}${change.toFixed(2)}%`;
}

function main() {
  console.log('📊 Performance Comparison Report\n');
  console.log('='.repeat(60));
  
  const baselineFile = process.argv[2] || 'baseline-load-test-summary.json';
  const currentFile = process.argv[3] || 'load-test-summary.json';
  
  const baseline = loadResults(baselineFile);
  const current = loadResults(currentFile);
  
  if (!baseline) {
    console.error(`❌ Baseline file not found: ${baselineFile}`);
    process.exit(1);
  }
  
  if (!current) {
    console.error(`❌ Current file not found: ${currentFile}`);
    process.exit(1);
  }
  
  console.log(`\nBaseline: ${baselineFile}`);
  console.log(`Current:  ${currentFile}\n`);
  
  const metrics = [
    {
      name: 'Avg Response Time',
      path: 'metrics.http_req_duration.values.avg',
      unit: 'ms',
      threshold: 10,
    },
    {
      name: 'P95 Response Time',
      path: 'metrics.http_req_duration.values.p(95)',
      unit: 'ms',
      threshold: 10,
    },
    {
      name: 'P99 Response Time',
      path: 'metrics.http_req_duration.values.p(99)',
      unit: 'ms',
      threshold: 10,
    },
    {
      name: 'Error Rate',
      path: 'metrics.http_req_failed.values.rate',
      unit: '%',
      threshold: 50,
      multiply: 100,
    },
    {
      name: 'Throughput',
      path: 'metrics.http_reqs.values.rate',
      unit: 'req/s',
      threshold: -10,
    },
  ];
  
  let hasRegression = false;
  
  console.log('Metric                    Baseline      Current       Change      Status');
  console.log('-'.repeat(80));
  
  metrics.forEach(metric => {
    const comparison = compareMetrics(baseline, current, metric.path);
    
    if (comparison === null) {
      console.log(`${metric.name.padEnd(25)} N/A`);
      return;
    }
    
    let baselineValue = comparison.baseline;
    let currentValue = comparison.current;
    
    if (metric.multiply) {
      baselineValue *= metric.multiply;
      currentValue *= metric.multiply;
    }
    
    const baselineStr = `${baselineValue.toFixed(2)}${metric.unit}`.padEnd(13);
    const currentStr = `${currentValue.toFixed(2)}${metric.unit}`.padEnd(13);
    const changeStr = formatChange(comparison.change).padEnd(11);
    
    let status = '✅ OK';
    if (metric.threshold > 0 && comparison.change > metric.threshold) {
      status = '⚠️ REGRESSION';
      hasRegression = true;
    } else if (metric.threshold < 0 && comparison.change < metric.threshold) {
      status = '⚠️ DEGRADATION';
      hasRegression = true;
    }
    
    console.log(`${metric.name.padEnd(25)} ${baselineStr} ${currentStr} ${changeStr} ${status}`);
  });
  
  console.log('='.repeat(80));
  
  if (hasRegression) {
    console.log('\n❌ Performance regression detected!');
    console.log('Review the changes and consider optimizations.\n');
    process.exit(1);
  } else {
    console.log('\n✅ No significant performance regression detected.\n');
    process.exit(0);
  }
}

main();
