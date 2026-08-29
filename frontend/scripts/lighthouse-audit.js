const fs = require('fs');
const path = require('path');
// `lighthouse` and `chrome-launcher` are ESM and heavy; they are required
// lazily inside runLighthouseAudit() so this module can be imported for its
// pure helpers (evaluateMetricBudgets) in a unit test without launching Chrome.

/**
 * Lighthouse Audit Script
 *
 * Validates Lighthouse category scores AND a Core Web Vitals performance
 * budget (LCP / CLS / TBT) for the landing route against the thresholds in
 * performance/config/thresholds.json (#1349).
 *
 * This must run against a real production build — `next build && next start`,
 * not `next dev` — so dev-mode overhead is not measured as if it were
 * production performance. CI (.github/workflows/accessibility.yml) already
 * builds and starts the app before invoking `npm run lighthouse`.
 *
 * Exit code is non-zero if any category is below its threshold OR any metric
 * exceeds its budget, so `npm run lighthouse` fails the build.
 */

// The landing page is the site root. `LIGHTHOUSE_PATH` allows auditing another
// route without changing the base URL.
const BASE_URL = (process.env.TEST_URL || 'http://localhost:3000').replace(/\/$/, '');
const LANDING_PATH = process.env.LIGHTHOUSE_PATH || '/';
const URL = `${BASE_URL}${LANDING_PATH}`;

// Default Core Web Vitals budget (the "good" thresholds): LCP <= 2.5 s,
// CLS <= 0.1, TBT <= 300 ms. Overridden by thresholds.json -> lighthouse.budgets.
const DEFAULT_BUDGETS = {
  'largest-contentful-paint': 2500,
  'cumulative-layout-shift': 0.1,
  'total-blocking-time': 300,
};

const DEFAULT_CATEGORY_THRESHOLDS = {
  performance: 90,
  accessibility: 95,
  'best-practices': 90,
  seo: 90,
};

// Load thresholds from performance/config/thresholds.json
function loadThresholds() {
  const thresholdsPath = path.join(__dirname, '../../performance/config/thresholds.json');
  try {
    const content = fs.readFileSync(thresholdsPath, 'utf8');
    const thresholds = JSON.parse(content);
    const lighthouse = thresholds.lighthouse || {};
    return {
      categories: {
        performance: lighthouse.performance,
        accessibility: lighthouse.accessibility,
        'best-practices': lighthouse['best-practices'],
        seo: lighthouse.seo,
      },
      budgets: lighthouse.budgets || DEFAULT_BUDGETS,
    };
  } catch (error) {
    console.warn('⚠️  Could not load thresholds.json, using defaults');
    return { categories: DEFAULT_CATEGORY_THRESHOLDS, budgets: DEFAULT_BUDGETS };
  }
}

const { categories: THRESHOLDS, budgets: BUDGETS } = loadThresholds();

// Human-readable label + unit per metric audit id.
const METRIC_LABELS = {
  'largest-contentful-paint': { label: 'LCP', unit: 'ms' },
  'cumulative-layout-shift': { label: 'CLS', unit: '' },
  'total-blocking-time': { label: 'TBT', unit: 'ms' },
};

/**
 * Compares Lighthouse metric audits against a budget map.
 * Pure (no I/O) so it can be unit-tested without launching Chrome.
 *
 * @param {Record<string, { numericValue?: number }>} audits - lhr.audits
 * @param {Record<string, number>} budgets - audit id -> max allowed numericValue
 * @returns {{ failures: Array, passes: Array }}
 */
function evaluateMetricBudgets(audits, budgets) {
  const failures = [];
  const passes = [];

  for (const [id, budget] of Object.entries(budgets || {})) {
    // `_`-prefixed keys (e.g. `_comment`) are documentation, not metrics.
    if (id.startsWith('_') || typeof budget !== 'number') continue;
    const audit = (audits || {})[id];
    if (!audit || typeof audit.numericValue !== 'number') {
      failures.push({ id, budget, actual: null, reason: 'metric missing from report' });
      continue;
    }
    const actual = audit.numericValue;
    const entry = { id, budget, actual };
    if (actual > budget) {
      failures.push(entry);
    } else {
      passes.push(entry);
    }
  }

  return { failures, passes };
}

async function runLighthouseAudit() {
  console.log('🚀 Starting Lighthouse audit (landing route + performance budget)...\n');
  console.log(`Testing URL: ${URL}\n`);

  const lighthouse = require('lighthouse');
  const chromeLauncher = require('chrome-launcher');

  let chrome;
  let results;

  try {
    // Launch Chrome
    chrome = await chromeLauncher.launch({
      chromeFlags: ['--headless', '--disable-gpu', '--no-sandbox'],
    });

    const options = {
      logLevel: 'info',
      output: ['html', 'json'],
      onlyCategories: ['performance', 'accessibility', 'best-practices', 'seo'],
      port: chrome.port,
    };

    // Run Lighthouse
    const runnerResult = await lighthouse(URL, options);
    results = runnerResult.lhr;

    // Save reports
    const reportsDir = path.join(__dirname, '../lighthouse-reports');
    if (!fs.existsSync(reportsDir)) {
      fs.mkdirSync(reportsDir, { recursive: true });
    }

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const htmlReport = runnerResult.report[0];
    const jsonReport = runnerResult.report[1];

    fs.writeFileSync(
      path.join(reportsDir, `lighthouse-${timestamp}.html`),
      htmlReport
    );
    fs.writeFileSync(
      path.join(reportsDir, `lighthouse-${timestamp}.json`),
      jsonReport
    );
    // Stable filename for downstream steps that don't want to glob timestamps
    // (accessibility.yml reads lighthouse-latest.json).
    fs.writeFileSync(path.join(reportsDir, 'lighthouse-latest.json'), jsonReport);

    console.log(`📊 Reports saved to: ${reportsDir}\n`);
    console.log(`   Audited: ${URL}\n`);

    // Analyze results
    console.log('='.repeat(60));
    console.log('LIGHTHOUSE AUDIT RESULTS');
    console.log('='.repeat(60));
    console.log();

    // Check all categories against thresholds
    const categories = ['performance', 'accessibility', 'best-practices', 'seo'];
    const failingCategories = [];
    const passingCategories = [];

    categories.forEach(category => {
      if (!results.categories[category]) {
        console.warn(`⚠️  Category '${category}' not found in results`);
        return;
      }

      const score = results.categories[category].score * 100;
      const threshold = THRESHOLDS[category] ?? DEFAULT_CATEGORY_THRESHOLDS[category];
      const status = score >= threshold ? '✅' : '❌';

      console.log(`${status} ${category.toUpperCase()}: ${score.toFixed(0)}/100 (threshold: ${threshold})`);

      if (score >= threshold) {
        passingCategories.push({ category, score, threshold });
      } else {
        failingCategories.push({ category, score, threshold });
      }
    });

    console.log();

    // Check the Core Web Vitals performance budget for the landing route.
    const { failures: budgetFailures, passes: budgetPasses } = evaluateMetricBudgets(
      results.audits,
      BUDGETS,
    );

    console.log('PERFORMANCE BUDGET (landing route):');
    console.log('-'.repeat(60));
    [...budgetPasses, ...budgetFailures]
      .sort((a, b) => a.id.localeCompare(b.id))
      .forEach(({ id, actual, budget }) => {
        const meta = METRIC_LABELS[id] || { label: id, unit: '' };
        const failed = budgetFailures.some((f) => f.id === id);
        const shown = actual === null ? 'n/a' : `${actual.toFixed(meta.unit === 'ms' ? 0 : 3)}${meta.unit}`;
        console.log(
          `${failed ? '❌' : '✅'} ${meta.label}: ${shown} (budget: ${budget}${meta.unit})`,
        );
      });
    console.log();

    // Show detailed failure information if any category fails
    if (failingCategories.length > 0) {
      console.log('FAILING CATEGORIES:');
      console.log('-'.repeat(60));

      failingCategories.forEach(({ category, score, threshold }) => {
        const diff = threshold - score;
        console.log(`\n❌ ${category.toUpperCase()}`);
        console.log(`   Threshold: ${threshold}`);
        console.log(`   Actual: ${score.toFixed(0)}`);
        console.log(`   Gap: -${diff.toFixed(0)} points`);
      });
    }

    if (budgetFailures.length > 0) {
      console.log('\nOVER BUDGET:');
      console.log('-'.repeat(60));
      budgetFailures.forEach(({ id, actual, budget, reason }) => {
        const meta = METRIC_LABELS[id] || { label: id, unit: '' };
        console.log(`\n❌ ${meta.label}`);
        console.log(`   Budget: ${budget}${meta.unit}`);
        console.log(`   Actual: ${actual === null ? reason : `${actual.toFixed(0)}${meta.unit}`}`);
      });
    }

    const passed = failingCategories.length === 0 && budgetFailures.length === 0;

    console.log('\n' + '='.repeat(60));
    console.log(
      passed
        ? '✅ PASSED: All categories meet thresholds and the landing page is within budget!'
        : '❌ FAILED: category threshold and/or performance budget not met!',
    );
    console.log('='.repeat(60));
    return passed;
  } catch (error) {
    console.error('❌ Error running Lighthouse audit:', error);
    return false;
  } finally {
    if (chrome) {
      await chrome.kill();
    }
  }
}

// Run audit if executed directly
if (require.main === module) {
  runLighthouseAudit()
    .then(passed => {
      process.exit(passed ? 0 : 1);
    })
    .catch(error => {
      console.error('Fatal error:', error);
      process.exit(1);
    });
}

module.exports = { runLighthouseAudit, evaluateMetricBudgets, loadThresholds };
