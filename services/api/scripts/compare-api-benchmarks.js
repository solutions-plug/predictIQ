#!/usr/bin/env node
/**
 * compare-api-benchmarks.js
 *
 * Compares current Criterion benchmark results against a stored baseline and
 * fails (exit 1) if any benchmark regresses by more than THRESHOLD percent.
 *
 * Usage:
 *   node compare-api-benchmarks.js <current.json> <baseline.json> [--threshold N]
 *
 * Both JSON files must be the flat map produced by parse-bench-output.js:
 *   { "bench_name": <median_ns>, ... }
 *
 * Exit codes:
 *   0 – no regression above threshold
 *   1 – one or more regressions detected
 */

'use strict';

const fs   = require('fs');
const path = require('path');

// ── CLI args ──────────────────────────────────────────────────────────────────
const args = process.argv.slice(2);
const currentFile  = args[0];
const baselineFile = args[1];
let threshold = 10; // default 10 %

const tIdx = args.indexOf('--threshold');
if (tIdx !== -1 && args[tIdx + 1]) {
  threshold = parseFloat(args[tIdx + 1]);
}

if (!currentFile || !baselineFile) {
  console.error('Usage: node compare-api-benchmarks.js <current.json> <baseline.json> [--threshold N]');
  process.exit(1);
}

if (!fs.existsSync(baselineFile)) {
  console.warn(`Baseline file not found: ${baselineFile}`);
  console.warn('Skipping comparison — this is expected on the first run.');
  process.exit(0);
}

// ── Load data ─────────────────────────────────────────────────────────────────
const current  = JSON.parse(fs.readFileSync(currentFile,  'utf8'));
const baseline = JSON.parse(fs.readFileSync(baselineFile, 'utf8'));

// ── Compare ───────────────────────────────────────────────────────────────────
let hasRegression = false;
const rows = [];

for (const [name, baselineNs] of Object.entries(baseline)) {
  if (!(name in current)) {
    rows.push({ status: '⚠️ ', name, note: 'missing from current run' });
    continue;
  }

  const currentNs  = current[name];
  const changePct  = ((currentNs - baselineNs) / baselineNs) * 100;
  const regressed  = changePct > threshold;

  if (regressed) hasRegression = true;

  rows.push({
    status : regressed ? '❌' : '✅',
    name,
    baseline: `${baselineNs.toFixed(1)} ns`,
    current : `${currentNs.toFixed(1)} ns`,
    change  : `${changePct > 0 ? '+' : ''}${changePct.toFixed(1)}%`,
  });
}

// Also report new benchmarks not in baseline.
for (const name of Object.keys(current)) {
  if (!(name in baseline)) {
    rows.push({ status: '🆕', name, note: 'new benchmark (no baseline yet)' });
  }
}

// ── Report ────────────────────────────────────────────────────────────────────
console.log('\n## API Criterion Benchmark Comparison\n');
console.log(`Threshold: ${threshold}% regression\n`);

const colW = Math.max(60, ...rows.map(r => r.name.length + 2));

console.log(
  'Status'.padEnd(6) +
  'Benchmark'.padEnd(colW) +
  'Baseline'.padEnd(16) +
  'Current'.padEnd(16) +
  'Change'
);
console.log('-'.repeat(colW + 40));

for (const row of rows) {
  if (row.note) {
    console.log(`${row.status}  ${row.name.padEnd(colW)}${row.note}`);
  } else {
    console.log(
      `${row.status}  ${row.name.padEnd(colW)}` +
      `${row.baseline.padEnd(16)}${row.current.padEnd(16)}${row.change}`
    );
  }
}

console.log('');

if (hasRegression) {
  console.error(`\n❌ One or more benchmarks regressed by more than ${threshold}%. CI failing.\n`);
  process.exit(1);
} else {
  console.log(`\n✅ All benchmarks within the ${threshold}% regression threshold.\n`);
}
