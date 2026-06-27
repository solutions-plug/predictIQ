const lighthouse = require('lighthouse');
const chromeLauncher = require('chrome-launcher');
const fs = require('fs');
const path = require('path');

/**
 * Lighthouse Audit Script
 * Validates Lighthouse scores against configured thresholds
 */

const URL = process.env.TEST_URL || 'http://localhost:3000';

// Load thresholds from performance/config/thresholds.json
function loadThresholds() {
  const thresholdsPath = path.join(__dirname, '../../performance/config/thresholds.json');
  try {
    const content = fs.readFileSync(thresholdsPath, 'utf8');
    const thresholds = JSON.parse(content);
    return thresholds.lighthouse || {};
  } catch (error) {
    console.warn('⚠️  Could not load thresholds.json, using defaults');
    return {
      performance: 90,
      accessibility: 95,
      'best-practices': 90,
      seo: 90,
    };
  }
}

const THRESHOLDS = loadThresholds();

async function runLighthouseAudit() {
  console.log('🚀 Starting Lighthouse Accessibility Audit...\n');
  console.log(`Testing URL: ${URL}\n`);

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

    console.log(`📊 Reports saved to: ${reportsDir}\n`);

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
      const threshold = THRESHOLDS[category];
      const status = score >= threshold ? '✅' : '❌';

      console.log(`${status} ${category.toUpperCase()}: ${score.toFixed(0)}/100 (threshold: ${threshold})`);

      if (score >= threshold) {
        passingCategories.push({ category, score, threshold });
      } else {
        failingCategories.push({ category, score, threshold });
      }
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

      console.log('\n' + '='.repeat(60));
      console.log('❌ FAILED: One or more categories below threshold!');
      console.log('='.repeat(60));
      return false;
    }

    // All categories passing
    console.log('='.repeat(60));
    console.log('✅ PASSED: All categories meet thresholds!');
    console.log('='.repeat(60));
    return true;
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

module.exports = { runLighthouseAudit };
