#!/usr/bin/env node

/**
 * Check SLO compliance before deployment
 * Queries Prometheus for current SLO metrics and blocks deployment if error budget exhausted
 */

const https = require('https');
const fs = require('fs');
const path = require('path');

const PROMETHEUS_URL = process.env.PROMETHEUS_URL || 'http://prometheus:9090';
const SLO_CONFIG = path.join(__dirname, '../config/slo.json');
const ALLOW_OVERRIDE = process.env.CONFIRM_SLO_OVERRIDE === 'yes';

async function queryPrometheus(query) {
  return new Promise((resolve, reject) => {
    const url = new URL(`${PROMETHEUS_URL}/api/v1/query`);
    url.searchParams.append('query', query);

    const protocol = url.protocol === 'https:' ? https : require('http');
    
    protocol.get(url, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          const result = JSON.parse(data);
          if (result.status === 'success') {
            resolve(result.data.result);
          } else {
            reject(new Error(`Prometheus error: ${result.error}`));
          }
        } catch (e) {
          reject(e);
        }
      });
    }).on('error', reject);
  });
}

async function checkSLOCompliance() {
  try {
    const sloConfig = JSON.parse(fs.readFileSync(SLO_CONFIG, 'utf8'));
    
    console.log('🔍 Checking SLO compliance...\n');
    
    let allCompliant = true;
    const results = [];

    for (const [sloName, sloTarget] of Object.entries(sloConfig.slos)) {
      try {
        // Query error budget remaining
        const query = `slo:${sloName}:error_budget_remaining`;
        const result = await queryPrometheus(query);
        
        if (result.length === 0) {
          console.warn(`⚠️  No data for SLO: ${sloName}`);
          continue;
        }

        const errorBudgetRemaining = parseFloat(result[0].value[1]);
        const compliant = errorBudgetRemaining > 0;
        
        results.push({
          slo: sloName,
          target: sloTarget,
          errorBudgetRemaining,
          compliant
        });

        const icon = compliant ? '✅' : '❌';
        console.log(`${icon} ${sloName}`);
        console.log(`   Target: ${sloTarget}%`);
        console.log(`   Error Budget Remaining: ${errorBudgetRemaining.toFixed(2)}%\n`);

        if (!compliant) {
          allCompliant = false;
        }
      } catch (error) {
        console.error(`❌ Error checking ${sloName}: ${error.message}`);
        allCompliant = false;
      }
    }

    // Print summary
    console.log('─'.repeat(50));
    if (allCompliant) {
      console.log('✅ All SLOs compliant - deployment allowed\n');
      return true;
    } else {
      console.log('❌ SLO compliance check failed\n');
      
      if (ALLOW_OVERRIDE) {
        console.log('⚠️  Override enabled via CONFIRM_SLO_OVERRIDE=yes');
        console.log('   Proceeding with deployment (requires approval)\n');
        return true;
      } else {
        console.log('🛑 Deployment blocked - error budget exhausted');
        console.log('   Set CONFIRM_SLO_OVERRIDE=yes to override (requires approval)\n');
        return false;
      }
    }
  } catch (error) {
    console.error(`Fatal error: ${error.message}`);
    process.exit(1);
  }
}

checkSLOCompliance().then(success => {
  process.exit(success ? 0 : 1);
});
