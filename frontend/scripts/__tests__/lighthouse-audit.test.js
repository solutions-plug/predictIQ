const path = require('path');
const { evaluateMetricBudgets, loadThresholds } = require('../lighthouse-audit');

/**
 * #1349 — `npm run lighthouse` must fail the build when the landing page's
 * LCP (or CLS / TBT) exceeds the configured budget. The Chrome run itself
 * can't execute in Jest, so the pass/fail decision is factored into the pure
 * `evaluateMetricBudgets` helper and asserted here.
 */

const BUDGETS = {
  'largest-contentful-paint': 2500,
  'total-blocking-time': 300,
  'cumulative-layout-shift': 0.1,
};

function auditsWith({ lcp, tbt, cls }) {
  return {
    'largest-contentful-paint': { numericValue: lcp },
    'total-blocking-time': { numericValue: tbt },
    'cumulative-layout-shift': { numericValue: cls },
  };
}

describe('evaluateMetricBudgets', () => {
  it('passes when every metric is within budget', () => {
    const { failures, passes } = evaluateMetricBudgets(
      auditsWith({ lcp: 1800, tbt: 120, cls: 0.02 }),
      BUDGETS,
    );
    expect(failures).toHaveLength(0);
    expect(passes.map((p) => p.id).sort()).toEqual(
      ['cumulative-layout-shift', 'largest-contentful-paint', 'total-blocking-time'],
    );
  });

  it('fails when LCP exceeds its budget', () => {
    const { failures } = evaluateMetricBudgets(
      auditsWith({ lcp: 4200, tbt: 100, cls: 0.01 }),
      BUDGETS,
    );
    expect(failures).toHaveLength(1);
    expect(failures[0]).toMatchObject({ id: 'largest-contentful-paint', budget: 2500, actual: 4200 });
  });

  it('fails when CLS exceeds its budget', () => {
    const { failures } = evaluateMetricBudgets(
      auditsWith({ lcp: 1000, tbt: 50, cls: 0.35 }),
      BUDGETS,
    );
    expect(failures.map((f) => f.id)).toContain('cumulative-layout-shift');
  });

  it('treats a metric missing from the report as a failure', () => {
    const { failures } = evaluateMetricBudgets({}, BUDGETS);
    expect(failures).toHaveLength(3);
    expect(failures.every((f) => f.actual === null)).toBe(true);
  });

  it('ignores `_`-prefixed documentation keys in the budget map', () => {
    const { failures, passes } = evaluateMetricBudgets(
      auditsWith({ lcp: 1000, tbt: 50, cls: 0.01 }),
      { ...BUDGETS, _comment: 'docs' },
    );
    expect(failures).toHaveLength(0);
    expect(passes).toHaveLength(3);
  });
});

describe('loadThresholds', () => {
  it('reads the landing-page Core Web Vitals budget from performance/config/thresholds.json', () => {
    const { budgets } = loadThresholds();
    expect(budgets['largest-contentful-paint']).toBe(2500);
    expect(budgets['total-blocking-time']).toBe(300);
    expect(budgets['cumulative-layout-shift']).toBe(0.1);
  });

  it('points at the repo-level performance config', () => {
    // Guards against the reports-dir / config-path relativity regressing.
    const configPath = path.join(__dirname, '../../../performance/config/thresholds.json');
    expect(require('fs').existsSync(configPath)).toBe(true);
  });
});
