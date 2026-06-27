#!/usr/bin/env node

/**
 * Error Budget Calculator
 * 
 * Calculates SLO compliance and error budget consumption based on metrics.
 * Supports multiple SLOs and generates reports.
 * Persists error budget snapshots for historical tracking.
 */

const fs = require('fs');
const path = require('path');

// Load SLO configuration
const sloConfig = JSON.parse(
  fs.readFileSync(path.join(__dirname, '../config/slo.json'), 'utf8')
);

const BASELINES_DIR = path.join(__dirname, '../.performance-baselines');

/**
 * Save error budget snapshot with timestamp
 * @param {Array} results - Error budget calculation results
 * @returns {string} Path to saved snapshot
 */
function saveErrorBudgetSnapshot(results) {
  if (!fs.existsSync(BASELINES_DIR)) {
    fs.mkdirSync(BASELINES_DIR, { recursive: true });
  }

  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const snapshotFile = path.join(BASELINES_DIR, `error-budget-${timestamp}.json`);
  
  const snapshot = {
    timestamp: new Date().toISOString(),
    results,
    summary: {
      total_slos: results.length,
      healthy: results.filter(r => r.status === 'healthy').length,
      warning: results.filter(r => r.status === 'warning').length,
      alert: results.filter(r => r.status === 'alert').length,
      critical: results.filter(r => r.status === 'critical').length,
      emergency: results.filter(r => r.status === 'emergency').length,
    },
  };
  
  fs.writeFileSync(snapshotFile, JSON.stringify(snapshot, null, 2));
  return snapshotFile;
}

/**
 * Load all error budget snapshots for trend analysis
 * @returns {Array} Array of snapshots sorted by timestamp
 */
function loadErrorBudgetHistory() {
  if (!fs.existsSync(BASELINES_DIR)) {
    return [];
  }

  const files = fs.readdirSync(BASELINES_DIR)
    .filter(f => f.startsWith('error-budget-') && f.endsWith('.json'))
    .sort();

  return files.map(file => {
    try {
      return JSON.parse(fs.readFileSync(path.join(BASELINES_DIR, file), 'utf8'));
    } catch (e) {
      console.error(`Failed to parse ${file}:`, e.message);
      return null;
    }
  }).filter(Boolean);
}

/**
 * Calculate error budget trend over N runs
 * @param {number} runs - Number of recent runs to analyze
 * @returns {Object} Trend analysis
 */
function calculateErrorBudgetTrend(runs = 10) {
  const history = loadErrorBudgetHistory();
  const recent = history.slice(-runs);

  if (recent.length === 0) {
    return { message: 'No historical data available' };
  }

  const trends = {};
  
  // Analyze each SLO
  recent.forEach(snapshot => {
    snapshot.results.forEach(result => {
      if (!trends[result.slo_name]) {
        trends[result.slo_name] = {
          slo_name: result.slo_name,
          samples: [],
          avg_remaining: 0,
          min_remaining: 100,
          max_remaining: 0,
          trend: 'stable',
        };
      }
      
      const remaining = parseFloat(result.error_budget_remaining);
      trends[result.slo_name].samples.push({
        timestamp: snapshot.timestamp,
        remaining,
      });
      trends[result.slo_name].min_remaining = Math.min(trends[result.slo_name].min_remaining, remaining);
      trends[result.slo_name].max_remaining = Math.max(trends[result.slo_name].max_remaining, remaining);
    });
  });

  // Calculate averages and trends
  Object.keys(trends).forEach(sloName => {
    const data = trends[sloName];
    data.avg_remaining = (data.samples.reduce((sum, s) => sum + s.remaining, 0) / data.samples.length).toFixed(2);
    
    // Determine trend direction
    if (data.samples.length >= 2) {
      const first = data.samples[0].remaining;
      const last = data.samples[data.samples.length - 1].remaining;
      const change = last - first;
      
      if (change < -5) {
        data.trend = 'degrading';
      } else if (change > 5) {
        data.trend = 'improving';
      } else {
        data.trend = 'stable';
      }
    }
  });

  return trends;
}

/**
 * Calculate error budget for a given SLO
 * @param {Object} slo - SLO configuration
 * @param {Object} metrics - Actual metrics data
 * @returns {Object} Error budget calculation results
 */
function calculateErrorBudget(slo, metrics) {
  const target = slo.target || 100;
  const errorBudgetPercent = slo.error_budget_percent || (100 - target);
  
  // Calculate actual performance
  const actualPerformance = metrics.success_rate || 0;
  const actualErrors = 100 - actualPerformance;
  
  // Calculate error budget consumption
  const errorBudgetConsumed = (actualErrors / errorBudgetPercent) * 100;
  const errorBudgetRemaining = Math.max(0, 100 - errorBudgetConsumed);
  
  // Calculate burn rate (how fast we're consuming the budget)
  const windowDays = parseWindowDays(slo.measurement_window);
  const burnRate = errorBudgetConsumed / windowDays;
  
  // Determine status
  let status = 'healthy';
  let action = 'Normal operations';
  
  if (errorBudgetRemaining <= 0) {
    status = 'emergency';
    action = 'Emergency - rollback recent changes';
  } else if (errorBudgetRemaining <= 10) {
    status = 'critical';
    action = 'Critical - freeze all deployments';
  } else if (errorBudgetRemaining <= 25) {
    status = 'alert';
    action = 'Alert - freeze non-critical deployments';
  } else if (errorBudgetRemaining <= 50) {
    status = 'warning';
    action = 'Warning - review recent changes';
  }
  
  return {
    slo_name: metrics.name,
    target,
    actual_performance: actualPerformance.toFixed(2),
    error_budget_percent: errorBudgetPercent,
    error_budget_consumed: errorBudgetConsumed.toFixed(2),
    error_budget_remaining: errorBudgetRemaining.toFixed(2),
    burn_rate: burnRate.toFixed(2),
    status,
    action,
    measurement_window: slo.measurement_window,
  };
}

/**
 * Parse measurement window string to days
 * @param {string} window - Window string like "30d", "7d"
 * @returns {number} Number of days
 */
function parseWindowDays(window) {
  const match = window.match(/(\d+)d/);
  return match ? parseInt(match[1]) : 30;
}

/**
 * Check burn rate alerts
 * @param {Object} slo - SLO configuration
 * @param {Object} metrics - Metrics data with time windows
 * @returns {Array} Alert conditions
 */
function checkBurnRateAlerts(slo, metrics) {
  const alerts = [];
  const burnRateConfig = sloConfig.burn_rate_alerts;
  
  // Fast burn check (1h/6h windows)
  if (metrics.burn_rate_1h && metrics.burn_rate_1h > burnRateConfig.fast_burn.burn_rate_threshold) {
    alerts.push({
      severity: 'critical',
      type: 'fast_burn',
      message: `Fast burn detected: ${metrics.burn_rate_1h.toFixed(2)}x (threshold: ${burnRateConfig.fast_burn.burn_rate_threshold}x)`,
      window: '1h',
      description: burnRateConfig.fast_burn.description,
    });
  }
  
  // Slow burn check (6h/24h windows)
  if (metrics.burn_rate_6h && metrics.burn_rate_6h > burnRateConfig.slow_burn.burn_rate_threshold) {
    alerts.push({
      severity: 'warning',
      type: 'slow_burn',
      message: `Slow burn detected: ${metrics.burn_rate_6h.toFixed(2)}x (threshold: ${burnRateConfig.slow_burn.burn_rate_threshold}x)`,
      window: '6h',
      description: burnRateConfig.slow_burn.description,
    });
  }
  
  return alerts;
}

/**
 * Generate SLO report
 * @param {Array} results - Array of error budget calculations
 * @returns {string} Formatted report
 */
function generateReport(results) {
  const timestamp = new Date().toISOString();
  
  let report = `
╔════════════════════════════════════════════════════════════════════════════╗
║                         SLO COMPLIANCE REPORT                              ║
║                    Generated: ${timestamp}                    ║
╚════════════════════════════════════════════════════════════════════════════╝

`;

  results.forEach(result => {
    const statusEmoji = {
      healthy: '✅',
      warning: '⚠️',
      alert: '🚨',
      critical: '🔴',
      emergency: '💀',
    }[result.status] || '❓';
    
    report += `
${statusEmoji} ${result.slo_name}
${'─'.repeat(80)}
Target:                 ${result.target}%
Actual Performance:     ${result.actual_performance}%
Error Budget:           ${result.error_budget_percent}%
Budget Consumed:        ${result.error_budget_consumed}%
Budget Remaining:       ${result.error_budget_remaining}%
Burn Rate:              ${result.burn_rate}% per day
Status:                 ${result.status.toUpperCase()}
Action Required:        ${result.action}
Measurement Window:     ${result.measurement_window}

`;
  });
  
  // Summary
  const healthyCount = results.filter(r => r.status === 'healthy').length;
  const warningCount = results.filter(r => r.status === 'warning').length;
  const alertCount = results.filter(r => r.status === 'alert').length;
  const criticalCount = results.filter(r => r.status === 'critical').length;
  const emergencyCount = results.filter(r => r.status === 'emergency').length;
  
  report += `
╔════════════════════════════════════════════════════════════════════════════╗
║                              SUMMARY                                       ║
╚════════════════════════════════════════════════════════════════════════════╝

Total SLOs:             ${results.length}
✅ Healthy:             ${healthyCount}
⚠️  Warning:            ${warningCount}
🚨 Alert:               ${alertCount}
🔴 Critical:            ${criticalCount}
💀 Emergency:           ${emergencyCount}

`;

  return report;
}

/**
 * Main execution
 */
function main() {
  // Example metrics data (in production, this would come from Prometheus/Grafana)
  const exampleMetrics = [
    {
      name: 'API Availability',
      success_rate: 99.95,
      burn_rate_1h: 2.0,
      burn_rate_6h: 1.5,
    },
    {
      name: 'API Latency P95',
      success_rate: 98.5,
      burn_rate_1h: 5.0,
      burn_rate_6h: 3.0,
    },
    {
      name: 'API Latency P99',
      success_rate: 99.2,
      burn_rate_1h: 8.0,
      burn_rate_6h: 4.0,
    },
    {
      name: 'Database Query Latency',
      success_rate: 97.0,
      burn_rate_1h: 12.0,
      burn_rate_6h: 8.0,
    },
    {
      name: 'Cache Availability',
      success_rate: 99.98,
      burn_rate_1h: 1.0,
      burn_rate_6h: 0.5,
    },
  ];
  
  // Calculate error budgets
  const results = [];
  const slos = sloConfig.service_level_objectives;
  
  exampleMetrics.forEach((metrics, index) => {
    const sloKey = Object.keys(slos)[index];
    if (sloKey) {
      const slo = slos[sloKey];
      const result = calculateErrorBudget(slo, metrics);
      results.push(result);
      
      // Check burn rate alerts
      const alerts = checkBurnRateAlerts(slo, metrics);
      if (alerts.length > 0) {
        console.error(`\n🚨 ALERTS for ${metrics.name}:`);
        alerts.forEach(alert => {
          console.error(`  [${alert.severity.toUpperCase()}] ${alert.message}`);
          console.error(`  ${alert.description}`);
        });
      }
    }
  });
  
  // Generate and display report
  const report = generateReport(results);
  console.log(report);
  
  // Save error budget snapshot for historical tracking
  const snapshotPath = saveErrorBudgetSnapshot(results);
  console.log(`\n✓ Error budget snapshot saved to: ${snapshotPath}`);
  
  // Save report to file
  const reportPath = path.join(__dirname, '../reports/slo-report.txt');
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(reportPath, report);
  console.log(`Report saved to: ${reportPath}`);
  
  // Calculate and display trend analysis
  const trends = calculateErrorBudgetTrend(10);
  if (trends.message) {
    console.log(`\n${trends.message}`);
  } else {
    console.log('\n📊 Error Budget Trend (Last 10 Runs):');
    console.log('─'.repeat(80));
    Object.values(trends).forEach(trend => {
      const trendEmoji = {
        improving: '📈',
        degrading: '📉',
        stable: '➡️',
      }[trend.trend] || '❓';
      console.log(`${trendEmoji} ${trend.slo_name}`);
      console.log(`   Avg Remaining: ${trend.avg_remaining}% | Min: ${trend.min_remaining.toFixed(2)}% | Max: ${trend.max_remaining.toFixed(2)}%`);
    });
  }
  
  // Exit with error code if any SLO is in critical/emergency state
  const hasCritical = results.some(r => ['critical', 'emergency'].includes(r.status));
  process.exit(hasCritical ? 1 : 0);
}

// Run if executed directly
if (require.main === module) {
  main();
}

module.exports = {
  calculateErrorBudget,
  checkBurnRateAlerts,
  generateReport,
  saveErrorBudgetSnapshot,
  loadErrorBudgetHistory,
  calculateErrorBudgetTrend,
};
