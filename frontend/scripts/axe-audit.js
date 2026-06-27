const { AxePuppeteer } = require('@axe-core/puppeteer');
const puppeteer = require('puppeteer');
const fs = require('fs');
const path = require('path');

/**
 * Axe-core Accessibility Audit Script
 * Comprehensive WCAG 2.1 AA validation
 */

const URL = process.env.TEST_URL || 'http://localhost:3000';
const WCAG_LEVEL = 'wcag2aa'; // or 'wcag21aa'

async function runAxeAudit() {
  console.log('🔍 Starting Axe-core Accessibility Audit...\n');
  console.log(`Testing URL: ${URL}`);
  console.log(`WCAG Level: ${WCAG_LEVEL}\n`);

  let browser;
  let violations = [];

  try {
    // Launch browser
    browser = await puppeteer.launch({
      headless: 'new',
      args: ['--no-sandbox', '--disable-setuid-sandbox'],
    });

    const page = await browser.newPage();
    await page.setBypassCSP(true);

    // Navigate to page
    console.log('Loading page...');
    await page.goto(URL, { waitUntil: 'networkidle0' });

    // Run axe
    console.log('Running axe-core analysis...\n');
    const results = await new AxePuppeteer(page)
      .withTags([WCAG_LEVEL, 'best-practice'])
      .analyze();

    violations = results.violations;

    // Save results
    const reportsDir = path.join(__dirname, '../axe-reports');
    if (!fs.existsSync(reportsDir)) {
      fs.mkdirSync(reportsDir, { recursive: true });
    }

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    fs.writeFileSync(
      path.join(reportsDir, `axe-report-${timestamp}.json`),
      JSON.stringify(results, null, 2)
    );

    console.log(`📊 Report saved to: ${reportsDir}\n`);

    // Display results
    console.log('='.repeat(60));
    console.log('AXE-CORE ACCESSIBILITY AUDIT RESULTS');
    console.log('='.repeat(60));
    console.log(`\nViolations Found: ${violations.length}`);
    console.log(`Passes: ${results.passes.length}`);
    console.log(`Incomplete: ${results.incomplete.length}\n`);

    if (violations.length === 0) {
      console.log('✅ No accessibility violations found!');
      console.log('='.repeat(60));
      return true;
    }

    // Group violations by impact
    const critical = violations.filter(v => v.impact === 'critical');
    const serious = violations.filter(v => v.impact === 'serious');
    const moderate = violations.filter(v => v.impact === 'moderate');
    const minor = violations.filter(v => v.impact === 'minor');

    console.log('VIOLATIONS BY IMPACT:');
    console.log(`  🔴 Critical: ${critical.length}`);
    console.log(`  🟠 Serious: ${serious.length}`);
    console.log(`  🟡 Moderate: ${moderate.length}`);
    console.log(`  🟢 Minor: ${minor.length}\n`);

    // Check for critical and serious violations (these will cause failure)
    const criticalOrSerious = [...critical, ...serious];

    if (criticalOrSerious.length > 0) {
      console.log('CRITICAL/SERIOUS VIOLATIONS (FAILING):');
      console.log('-'.repeat(60));

      criticalOrSerious.forEach((violation, index) => {
        const impactEmoji = violation.impact === 'critical' ? '🔴' : '🟠';

        console.log(`\n${index + 1}. ${impactEmoji} ${violation.id.toUpperCase()}`);
        console.log(`   Impact: ${violation.impact}`);
        console.log(`   Description: ${violation.description}`);
        console.log(`   Help: ${violation.help}`);
        console.log(`   URL: ${violation.helpUrl}`);
        console.log(`   WCAG: ${violation.tags.filter(t => t.startsWith('wcag')).join(', ')}`);
        console.log(`   Affected elements: ${violation.nodes.length}`);

        violation.nodes.slice(0, 3).forEach((node, nodeIndex) => {
          console.log(`\n   Element ${nodeIndex + 1}:`);
          console.log(`     HTML: ${node.html.substring(0, 100)}...`);
          console.log(`     Target: ${node.target.join(' > ')}`);
          console.log(`     Fix: ${node.failureSummary}`);
        });

        if (violation.nodes.length > 3) {
          console.log(`\n   ... and ${violation.nodes.length - 3} more elements`);
        }
      });

      console.log('\n' + '='.repeat(60));
      console.log('❌ FAILED: Critical or serious accessibility violations found!');
      console.log('='.repeat(60));

      return false;
    }

    // If only moderate/minor violations, pass but display them
    if (violations.length > 0) {
      console.log('OTHER VIOLATIONS (NON-CRITICAL):');
      console.log('-'.repeat(60));

      [...moderate, ...minor].forEach((violation, index) => {
        const impactEmoji = {
          moderate: '🟡',
          minor: '🟢',
        }[violation.impact] || '⚪';

        console.log(`\n${index + 1}. ${impactEmoji} ${violation.id.toUpperCase()}`);
        console.log(`   Impact: ${violation.impact}`);
        console.log(`   Description: ${violation.description}`);
      });
    }

    console.log('\n' + '='.repeat(60));
    console.log('✅ PASSED: No critical or serious violations found!');
    console.log('='.repeat(60));

    return true;
  } catch (error) {
    console.error('❌ Error running axe audit:', error);
    return false;
  } finally {
    if (browser) {
      await browser.close();
    }
  }
}

// Run audit if executed directly
if (require.main === module) {
  runAxeAudit()
    .then(passed => {
      process.exit(passed ? 0 : 1);
    })
    .catch(error => {
      console.error('Fatal error:', error);
      process.exit(1);
    });
}

module.exports = { runAxeAudit };
