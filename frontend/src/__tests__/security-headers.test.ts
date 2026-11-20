/** @jest-environment node */
import { NextRequest } from 'next/server';
import nextConfig from '../../next.config';
import { proxy } from '../proxy';

describe('Security Headers', () => {
  let headers: any;

  beforeAll(async () => {
    if (typeof nextConfig.headers === 'function') {
      const result = await nextConfig.headers();
      headers = result[0].headers;
    }
  });

  it('should have X-Content-Type-Options header set to nosniff', () => {
    const header = headers.find((h: any) => h.key === 'X-Content-Type-Options');
    expect(header).toBeDefined();
    expect(header.value).toBe('nosniff');
  });

  it('should have X-Frame-Options header set to DENY', () => {
    const header = headers.find((h: any) => h.key === 'X-Frame-Options');
    expect(header).toBeDefined();
    expect(header.value).toBe('DENY');
  });

  it('should have Referrer-Policy header set to strict-origin-when-cross-origin', () => {
    const header = headers.find((h: any) => h.key === 'Referrer-Policy');
    expect(header).toBeDefined();
    expect(header.value).toBe('strict-origin-when-cross-origin');
  });

  it('should have Permissions-Policy header configured', () => {
    const header = headers.find((h: any) => h.key === 'Permissions-Policy');
    expect(header).toBeDefined();
    expect(header.value).toContain('geolocation=()');
    expect(header.value).toContain('microphone=()');
    expect(header.value).toContain('camera=()');
  });

  // Content-Security-Policy is deliberately NOT asserted here: it is computed
  // per-request in src/proxy.ts (nonce + runtime API origin), not by
  // next.config.js's static headers(). See src/__tests__/proxy.test.ts for
  // the CSP that actually reaches the browser.
  it('should not define a static Content-Security-Policy here (owned by proxy.ts)', () => {
    const header = headers.find((h: any) => h.key === 'Content-Security-Policy');
    expect(header).toBeUndefined();
  });

  it('should have all required security headers', () => {
    const requiredHeaders = [
      'X-Content-Type-Options',
      'X-Frame-Options',
      'Referrer-Policy',
      'Permissions-Policy',
    ];

    const headerKeys = headers.map((h: any) => h.key);
    requiredHeaders.forEach(headerKey => {
      expect(headerKeys).toContain(headerKey);
    });
  });

  describe('Security headers: app/proxy interaction (#1928)', () => {
    const keys = () => (headers as any[]).map((h) => h.key.toLowerCase());

    it('emits Strict-Transport-Security from the app with a long max-age', () => {
      const hsts = headers.find((h: any) => h.key === 'Strict-Transport-Security');
      expect(hsts).toBeDefined();
      expect(hsts.value).toMatch(/max-age=(\d+)/);
      expect(Number(hsts.value.match(/max-age=(\d+)/)[1])).toBeGreaterThanOrEqual(31536000);
    });

    it('defines each static security header exactly once (no duplicates)', () => {
      const all = keys();
      expect(new Set(all).size).toBe(all.length);
    });

    it('does not set headers the proxy owns (CSP) in the static config', () => {
      expect(keys()).not.toContain('content-security-policy');
    });

    it('emits exactly one CSP and no duplicated security headers for a request already carrying forwarded headers', () => {
      const request = new NextRequest(new URL('/', 'http://localhost:3000'), {
        headers: {
          'x-forwarded-proto': 'https',
          'x-forwarded-for': '203.0.113.7',
          'x-forwarded-host': 'predictiq.app',
        },
      });
      const response = proxy(request);

      expect(response.headers.get('Content-Security-Policy')).toContain("default-src 'self'");
      // Headers.get() would comma-join duplicates; a single value must not repeat the directive.
      const csp = response.headers.get('Content-Security-Policy')!;
      expect(csp.match(/default-src/g)).toHaveLength(1);

      // Proxy must not duplicate headers that next.config.js already owns.
      for (const name of ['x-frame-options', 'x-content-type-options', 'strict-transport-security', 'referrer-policy', 'permissions-policy']) {
        expect(response.headers.get(name)).toBeNull();
      }
      // Forwarded headers must not be echoed back to the browser.
      expect(response.headers.get('x-forwarded-proto')).toBeNull();
      expect(response.headers.get('x-forwarded-for')).toBeNull();
    });
  });
});
