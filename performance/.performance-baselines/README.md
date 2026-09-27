# Performance Baselines

This directory stores performance test baselines per branch. Baselines are used to detect performance regressions in pull requests.

## Structure

Baseline files follow the naming convention:

```
{branch}-{test-name}-baseline.json
```

Examples:

- `main-load-test-baseline.json` — Load test baseline for main branch
- `main-cache-test-baseline.json` — Cache test baseline for main branch
- `main-blockchain-load-test-baseline.json` — Blockchain load test baseline for main branch
- `develop-load-test-baseline.json` — Load test baseline for develop branch

## How It Works

### On Main Branch (Push)

1. Performance tests run
2. Results are saved as baselines for the current branch
3. Baselines are committed and pushed to the repository

### On Pull Requests

1. Performance tests run against the PR code
2. Results are compared against the baseline from the target branch (usually `main`)
3. If any metric regresses by more than the threshold (default 10%), the check fails
4. A regression report is posted to the PR with details

## Regression Thresholds

Default threshold: **10%**

Metrics compared:

- **Avg Response Time** — Lower is better
- **P95 Response Time** — Lower is better
- **P99 Response Time** — Lower is better
- **Error Rate** — Lower is better
- **Throughput** — Higher is better
- **Cache Hit Rate** — Higher is better

## Manual Baseline Update

To manually update baselines for a branch:

```bash
cd performance
node scripts/compare-results.js --save-baseline --branch main
```

This will save all current test results as baselines for the specified branch.

## Comparing Against a Specific Branch

To compare results against a different branch's baseline:

```bash
cd performance
node scripts/compare-results.js --branch develop --threshold 15
```

This compares against the `develop` branch baseline with a 15% threshold.

## Baseline Files

Baseline files are JSON snapshots of test results and include:

- Metrics (response times, error rates, throughput)
- Cache statistics
- Timestamp of when the baseline was created

They are committed to the repository to ensure consistent regression detection across CI runs.

## Bootstrapping Baselines

This directory intentionally ships with no committed baseline files. Baselines are
**generated from a real run**, never hand-authored, so that regression detection is
based on genuine measurements rather than invented numbers.

To seed the initial baselines, run the performance suite on `main` and save the
results:

```bash
cd performance
node scripts/compare-results.js --save-baseline --branch main
```

This writes `main-{test-name}-baseline.json` files into this directory. Commit and
push them so subsequent pull requests have something to compare against.

> **Note:** Until at least one baseline file exists here, `compare-results.js`
> cannot detect regressions. It will emit a warning that there is *nothing to
> compare against* (distinct from a clean *no regression* result) so the missing
> baseline is never silently treated as a pass.
