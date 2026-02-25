const lighthouse = require('lighthouse');
const chromeLauncher = require('chrome-launcher');
const fs = require('fs');
const path = require('path');

/**
 * Lighthouse Accessibility Audit Script
 * Ensures 100% accessibility score
 */

const ACCESSIBILITY_THRESHOLD = 100;
const URL = process.env.TEST_URL || 'http://localhost:3000';

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
      onlyCategories: ['accessibility'],
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
    const accessibilityScore = results.categories.accessibility.score * 100;
    console.log('='.repeat(60));
    console.log('LIGHTHOUSE ACCESSIBILITY AUDIT RESULTS');
    console.log('='.repeat(60));
    console.log(`\nAccessibility Score: ${accessibilityScore}/100`);
    console.log(`Threshold: ${ACCESSIBILITY_THRESHOLD}/100\n`);

    // List all audits
    const audits = results.categories.accessibility.auditRefs;
    const passedAudits = [];
    const failedAudits = [];
    const manualAudits = [];

    audits.forEach(auditRef => {
      const audit = results.audits[auditRef.id];
      if (audit.score === null) {
        manualAudits.push({ id: auditRef.id, title: audit.title });
      } else if (audit.score === 1) {
        passedAudits.push({ id: auditRef.id, title: audit.title });
      } else {
        failedAudits.push({
          id: auditRef.id,
          title: audit.title,
          description: audit.description,
          score: audit.score,
        });
      }
    });

    console.log(`✅ Passed Audits: ${passedAudits.length}`);
    console.log(`❌ Failed Audits: ${failedAudits.length}`);
    console.log(`⚠️  Manual Audits Required: ${manualAudits.length}\n`);

    // Show failed audits
    if (failedAudits.length > 0) {
      console.log('FAILED AUDITS:');
      console.log('-'.repeat(60));
      failedAudits.forEach(audit => {
        console.log(`\n❌ ${audit.title}`);
        console.log(`   Score: ${(audit.score * 100).toFixed(0)}/100`);
        console.log(`   ${audit.description}`);
      });
      console.log();
    }

    // Show manual audits
    if (manualAudits.length > 0) {
      console.log('MANUAL AUDITS REQUIRED:');
      console.log('-'.repeat(60));
      manualAudits.forEach(audit => {
        console.log(`⚠️  ${audit.title}`);
      });
      console.log();
    }

    // Check if threshold is met
    console.log('='.repeat(60));
    if (accessibilityScore >= ACCESSIBILITY_THRESHOLD) {
      console.log('✅ PASSED: Accessibility score meets threshold!');
      console.log('='.repeat(60));
      return true;
    } else {
      console.log('❌ FAILED: Accessibility score below threshold!');
      console.log(`   Required: ${ACCESSIBILITY_THRESHOLD}`);
      console.log(`   Actual: ${accessibilityScore}`);
      console.log('='.repeat(60));
      return false;
    }
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
