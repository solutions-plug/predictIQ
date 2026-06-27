#!/usr/bin/env node

/**
 * Performance Regression Detection Script
 *
 * Compares current test results against baseline results stored per branch.
 * Baselines are stored in .performance-baselines/ directory.
 *
 * Usage:
 *   node compare-results.js [--save-baseline] [--branch <name>] [--threshold <pct>]
 *
 * Examples:
 *   node compare-results.js                    # Compare against baseline
 *   node compare-results.js --save-baseline    # Save current as baseline
 *   node compare-results.js --branch main      # Compare against main baseline
 *   node compare-results.js --threshold 15     # Use 15% regression threshold
 */

const fs = require("fs");
const path = require("path");

const REPORTS_DIR = path.join(__dirname, "..", "backend", "reports");
const BASELINES_DIR = path.join(__dirname, "..", ".performance-baselines");

// Parse CLI arguments
const args = process.argv.slice(2);
const saveBaseline = args.includes("--save-baseline");
const branchIdx = args.indexOf("--branch");
const branch =
  branchIdx !== -1
    ? args[branchIdx + 1]
    : process.env.GITHUB_REF_NAME || "main";
const thresholdIdx = args.indexOf("--threshold");
const regressionThreshold =
  thresholdIdx !== -1 ? parseFloat(args[thresholdIdx + 1]) : 10;

// Ensure baselines directory exists
if (!fs.existsSync(BASELINES_DIR)) {
  fs.mkdirSync(BASELINES_DIR, { recursive: true });
}

/**
 * Load test results from reports directory
 */
function loadResults(filename) {
  const filePath = path.join(REPORTS_DIR, filename);
  if (!fs.existsSync(filePath)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (e) {
    console.error(`Failed to parse ${filename}:`, e.message);
    return null;
  }
}

/**
 * Load baseline results for a specific branch
 */
function loadBaseline(testName, branchName) {
  const baselineFile = path.join(
    BASELINES_DIR,
    `${branchName}-${testName}-baseline.json`,
  );
  if (!fs.existsSync(baselineFile)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(baselineFile, "utf8"));
  } catch (e) {
    console.error(`Failed to parse baseline ${baselineFile}:`, e.message);
    return null;
  }
}

/**
 * Save current results as baseline for a branch
 */
function saveAsBaseline(testName, branchName, results) {
  const baselineFile = path.join(
    BASELINES_DIR,
    `${branchName}-${testName}-baseline.json`,
  );
  fs.writeFileSync(baselineFile, JSON.stringify(results, null, 2));
  console.log(`✓ Saved baseline: ${baselineFile}`);
}

/**
 * Compare two metric values and calculate regression percentage
 */
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
    regression: change > regressionThreshold,
  };
}

/**
 * Get nested object value by dot-notation path
 */
function getNestedValue(obj, path) {
  return path.split(".").reduce((current, key) => {
    return current && current[key] !== undefined ? current[key] : null;
  }, obj);
}

/**
 * Format percentage change with sign
 */
function formatChange(change) {
  const sign = change > 0 ? "+" : "";
  return `${sign}${change.toFixed(2)}%`;
}

/**
 * Define metrics to compare
 */
const METRICS = [
  {
    name: "Avg Response Time",
    path: "metrics.http_req_duration.values.avg",
    unit: "ms",
    threshold: regressionThreshold,
    direction: "lower-is-better",
  },
  {
    name: "P95 Response Time",
    path: "metrics.http_req_duration.values.p(95)",
    unit: "ms",
    threshold: regressionThreshold,
    direction: "lower-is-better",
  },
  {
    name: "P99 Response Time",
    path: "metrics.http_req_duration.values.p(99)",
    unit: "ms",
    threshold: regressionThreshold,
    direction: "lower-is-better",
  },
  {
    name: "Error Rate",
    path: "metrics.http_req_failed.values.rate",
    unit: "%",
    threshold: regressionThreshold,
    multiply: 100,
    direction: "lower-is-better",
  },
  {
    name: "Throughput",
    path: "metrics.http_reqs.values.rate",
    unit: "req/s",
    threshold: -regressionThreshold,
    direction: "higher-is-better",
  },
  {
    name: "Cache Hit Rate",
    path: "cache.hit_rate_pct",
    unit: "%",
    threshold: -regressionThreshold,
    direction: "higher-is-better",
  },
];

/**
 * Main comparison logic
 */
function compareResults(testName, currentResults) {
  const baseline = loadBaseline(testName, branch);

  if (!baseline) {
    console.log(
      `⚠️  No baseline found for branch '${branch}'. Skipping comparison.`,
    );
    return { hasRegression: false, regressions: [] };
  }

  console.log(
    `\n📊 Comparing ${testName} against baseline (branch: ${branch})`,
  );
  console.log("=".repeat(90));

  let hasRegression = false;
  const regressions = [];

  console.log(
    "Metric".padEnd(25) +
      "Baseline".padEnd(15) +
      "Current".padEnd(15) +
      "Change".padEnd(12) +
      "Status",
  );
  console.log("-".repeat(90));

  METRICS.forEach((metric) => {
    const comparison = compareMetrics(baseline, currentResults, metric.path);

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

    const baselineStr = `${baselineValue.toFixed(2)}${metric.unit}`.padEnd(15);
    const currentStr = `${currentValue.toFixed(2)}${metric.unit}`.padEnd(15);
    const changeStr = formatChange(comparison.change).padEnd(12);

    let status = "✅ OK";
    let isRegression = false;

    if (
      metric.direction === "lower-is-better" &&
      comparison.change > metric.threshold
    ) {
      status = "⚠️ REGRESSION";
      isRegression = true;
      hasRegression = true;
    } else if (
      metric.direction === "higher-is-better" &&
      comparison.change < metric.threshold
    ) {
      status = "⚠️ REGRESSION";
      isRegression = true;
      hasRegression = true;
    }

    if (isRegression) {
      regressions.push({
        metric: metric.name,
        baseline: baselineValue,
        current: currentValue,
        change: comparison.change,
      });
    }

    console.log(
      `${metric.name.padEnd(25)} ${baselineStr} ${currentStr} ${changeStr} ${status}`,
    );
  });

  console.log("=".repeat(90));

  return { hasRegression, regressions };
}

/**
 * Load and display error budget trend
 */
function displayErrorBudgetTrend() {
  try {
    const errorBudgetModule = require('./calculate-error-budget.js');
    const trends = errorBudgetModule.calculateErrorBudgetTrend(10);
    
    if (trends.message) {
      console.log(`\n${trends.message}`);
      return;
    }
    
    console.log('\n📊 Error Budget Trend (Last 10 Runs):');
    console.log('─'.repeat(90));
    
    Object.values(trends).forEach(trend => {
      const trendEmoji = {
        improving: '📈',
        degrading: '📉',
        stable: '➡️',
      }[trend.trend] || '❓';
      
      console.log(`${trendEmoji} ${trend.slo_name}`);
      console.log(`   Avg Remaining: ${trend.avg_remaining}% | Min: ${trend.min_remaining.toFixed(2)}% | Max: ${trend.max_remaining.toFixed(2)}%`);
    });
  } catch (e) {
    console.log('\n⚠️  Could not load error budget trend:', e.message);
  }
}

/**
 * Generate markdown report for PR comment
 */
function generateMarkdownReport(testResults) {
  let markdown = "## 📊 Performance Regression Detection\n\n";

  let hasAnyRegression = false;
  const allRegressions = [];

  Object.entries(testResults).forEach(([testName, result]) => {
    if (result.hasRegression) {
      hasAnyRegression = true;
      result.regressions.forEach((r) => {
        allRegressions.push({ test: testName, ...r });
      });
    }
  });

  if (hasAnyRegression) {
    markdown += "### ⚠️ Performance Regressions Detected\n\n";
    markdown += "| Test | Metric | Baseline | Current | Change |\n";
    markdown += "|------|--------|----------|---------|--------|\n";

    allRegressions.forEach((r) => {
      markdown += `| ${r.test} | ${r.metric} | ${r.baseline.toFixed(2)} | ${r.current.toFixed(2)} | ${formatChange(r.change)} |\n`;
    });

    markdown +=
      "\n**Action Required:** Review the changes and consider optimizations.\n";
  } else {
    markdown += "### ✅ No Performance Regressions\n\n";
    markdown += "All metrics are within acceptable thresholds.\n";
  }

  markdown += `\n**Threshold:** ${regressionThreshold}% | **Branch:** ${branch}\n`;

  return markdown;
}

/**
 * Main execution
 */
function main() {
  console.log("🔍 Performance Regression Detection\n");

  if (saveBaseline) {
    console.log(
      `💾 Saving current results as baseline for branch '${branch}'...\n`,
    );

    const testFiles = [
      "load-test-summary.json",
      "cache-test-summary.json",
      "blockchain-load-test-summary.json",
      "stress-test-summary.json",
    ];

    testFiles.forEach((file) => {
      const testName = file.replace("-summary.json", "");
      const results = loadResults(file);

      if (results) {
        saveAsBaseline(testName, branch, results);
      }
    });

    console.log("\n✅ Baselines saved successfully");
    process.exit(0);
  }

  // Compare mode
  const testFiles = [
    "load-test-summary.json",
    "cache-test-summary.json",
    "blockchain-load-test-summary.json",
  ];

  const testResults = {};
  let hasAnyRegression = false;

  testFiles.forEach((file) => {
    const testName = file.replace("-summary.json", "");
    const current = loadResults(file);

    if (current) {
      const result = compareResults(testName, current);
      testResults[testName] = result;

      if (result.hasRegression) {
        hasAnyRegression = true;
      }
    } else {
      console.log(`⚠️  Results file not found: ${file}`);
    }
  });

  // Generate markdown report
  const markdownReport = generateMarkdownReport(testResults);
  const reportFile = path.join(REPORTS_DIR, "regression-report.md");
  fs.writeFileSync(reportFile, markdownReport);
  console.log(`\n📄 Regression report saved: ${reportFile}`);

  // Display error budget trend
  displayErrorBudgetTrend();

  if (hasAnyRegression) {
    console.log("\n❌ Performance regression detected!");
    console.log("Review the changes and consider optimizations.\n");
    process.exit(1);
  } else {
    console.log("\n✅ No significant performance regression detected.\n");
    process.exit(0);
  }
}

main();
