'use client';

import * as React from 'react';
import { useEffect } from 'react';
import { reportAccessibility } from '../lib/reportAccessibility';

/**
 * Renders nothing in the DOM. On mount — and only in a development client
 * build — it wires up @axe-core/react so axe violations show up in the
 * DevTools console as you develop.
 *
 * Rendered directly inside the root `<body>` (see src/app/layout.tsx) so it is
 * part of the top-level React tree and shares the app's React/ReactDOM
 * instances with @axe-core/react. It is fully eliminated from the production
 * bundle: the NODE_ENV guard inside `reportAccessibility` becomes `false` at
 * build time and the guarded imports are tree-shaken out.
 */
export function AxeAccessibility() {
  useEffect(() => {
    void reportAccessibility(React);
  }, []);

  return null;
}

export default AxeAccessibility;