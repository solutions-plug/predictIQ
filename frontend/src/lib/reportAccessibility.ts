/**
 * Dev-only accessibility harness for @axe-core/react.
 *
 * @axe-core/react monkey-patches ReactDOM so it re-scans the committed DOM
 * after each render and logs any axe violations to the DevTools console. It
 * catches issues locally during development before they ever reach CI's
 * `accessibility.yml`, which runs the heavyweight axe/Lighthouse/pa11y audits.
 *
 * Everything here is a no-op under `next build`: Next.js statically replaces
 * `process.env.NODE_ENV === 'development'` with `false` at build time, so the
 * guarded branch — including the dynamic `@axe-core/react` and `react-dom`
 * imports — is dead-code-eliminated and tree-shaken out of the production
 * bundle. Only the unguarded harness (a handful of bytes) remains.
 */

// Ensure we only ever initialize axe once, so multiple client re-mounts
// (Fast Refresh, route transitions) don't patch ReactDOM repeatedly.
let initialized = false;

/**
 * Initialize @axe-core/react in a development client build.
 *
 * Must be called from a client component that shares the app's React/ReactDOM
 * instances. `config` is forwarded to axe and lets callers restrict rules; the
 * default runs axe's standard WCAG rule set.
 */
export async function reportAccessibility(
  ReactModule: typeof import('react'),
  config?: Record<string, unknown>
): Promise<void> {
  if (process.env.NODE_ENV !== 'development') return;
  if (typeof window === 'undefined') return;
  if (initialized) return;

  initialized = true;

  const axe = await import('@axe-core/react');
  const ReactDOM = await import('react-dom');

  // (ReactModule, ReactDOM, timeoutMs, config) — timeout is how long axe waits
  // after a render before scanning, to avoid throttling during bursty updates.
  axe.default(ReactModule, ReactDOM, 1000, config);
}