#!/usr/bin/env node
/**
 * parse-bench-output.js
 *
 * Parses `cargo bench` (Criterion) stderr/stdout into a flat JSON map of
 * benchmark name → median nanoseconds.
 *
 * Usage:
 *   node parse-bench-output.js <input-file> <output-json>
 *
 * Criterion emits lines like:
 *   api_key_verify_constant_time/early_mismatch
 *                           time:   [12.123 ns 12.456 ns 12.789 ns]
 *
 * We capture the middle (median) estimate from the three-value bracket.
 */

'use strict';

const fs   = require('fs');
const path = require('path');

const [,, inputFile, outputFile] = process.argv;

if (!inputFile || !outputFile) {
  console.error('Usage: node parse-bench-output.js <input-file> <output-json>');
  process.exit(1);
}

const raw = fs.readFileSync(inputFile, 'utf8');

// State machine: remember the last benchmark name line, then look for `time:`.
const results = {};
let currentBench = null;

// Unit multipliers → convert everything to nanoseconds.
const toNs = { ns: 1, µs: 1e3, us: 1e3, ms: 1e6, s: 1e9 };

for (const line of raw.split('\n')) {
  // Criterion benchmark name line: no leading whitespace, ends with nothing special
  const nameMatch = line.match(/^([a-zA-Z0-9_/: -]+)\s*$/);
  if (nameMatch) {
    const candidate = nameMatch[1].trim();
    // Filter out non-benchmark lines (test result lines, etc.)
    if (!candidate.startsWith('test ') && !candidate.includes('FAILED') && candidate.length > 0) {
      currentBench = candidate;
    }
    continue;
  }

  // Criterion time line:  time:   [lo  median  hi]
  const timeMatch = line.match(/time:\s+\[[\d.]+ \S+\s+([\d.]+)\s+(\S+)\s+[\d.]+ \S+\]/);
  if (timeMatch && currentBench) {
    const value = parseFloat(timeMatch[1]);
    const unit  = timeMatch[2];
    const multiplier = toNs[unit] ?? 1;
    results[currentBench] = parseFloat((value * multiplier).toFixed(3));
    currentBench = null;
  }
}

if (Object.keys(results).length === 0) {
  console.warn('Warning: no benchmark results found in input file.');
  console.warn('Confirm that `cargo bench` output was captured correctly.');
}

fs.writeFileSync(outputFile, JSON.stringify(results, null, 2));
console.log(`Parsed ${Object.keys(results).length} benchmarks → ${outputFile}`);
