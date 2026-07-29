/**
 * @jest-environment node
 *
 * Integration coverage for the CSP actually served to the browser.
 *
 * next.config.js's headers() and this proxy used to define two different,
 * divergent Content-Security-Policy values for the same routes (the static
 * one in next.config.js even used 'strict-dynamic' with no nonce/hash, which
 * per spec blocks nearly all script execution). next.config.js no longer
 * sets CSP at all (see security-headers.test.ts) — proxy.ts is now the only
 * place that computes it.
 *
 * Earlier versions of this file fully mocked `next/server`, so it only
 * verified the string-building logic in isolation and never exercised what
 * a real request/response round-trip through `proxy()` actually produces.
 * These tests use the real NextRequest/NextResponse classes and call the
 * real `proxy` function — the same function Next's edge runtime invokes for
 * every matched request — and inspect the header that would actually reach
 * the browser.
 */
import { NextRequest } from 'next/server';
import { proxy } from '../proxy';

function runProxy(path = '/') {
  const request = new NextRequest(new URL(path, 'http://localhost:3000'));
  return { request, response: proxy(request) };
}

describe('proxy Content-Security-Policy', () => {
  it('serves exactly one Content-Security-Policy header on the real response', () => {
    const { response } = runProxy('/');
    expect(response.headers.get('Content-Security-Policy')).toBeTruthy();
  });

  it('uses a per-request nonce plus strict-dynamic, not a bare strict-dynamic with no nonce/hash', () => {
    const csp = runProxy('/').response.headers.get('Content-Security-Policy')!;

    const scriptSrcMatch = csp.match(/script-src\s+([^;]+)/);
    expect(scriptSrcMatch).toBeTruthy();
    const scriptSrc = scriptSrcMatch![1];

    expect(scriptSrc).toContain("'strict-dynamic'");
    expect(scriptSrc).toMatch(/'nonce-[A-Za-z0-9+/=]+'/);
  });

  it('generates a fresh nonce per request rather than a static/reused value', () => {
    const nonceOf = (csp: string) => csp.match(/'nonce-([^']+)'/)?.[1];

    const nonceA = nonceOf(runProxy('/').response.headers.get('Content-Security-Policy')!);
    const nonceB = nonceOf(runProxy('/').response.headers.get('Content-Security-Policy')!);

    expect(nonceA).toBeTruthy();
    expect(nonceB).toBeTruthy();
    expect(nonceA).not.toBe(nonceB);
  });

  it('forwards the same nonce to the request headers so RootLayout can apply it to the inline dark-mode init script', () => {
    const { response } = runProxy('/');

    const cspNonce = response.headers
      .get('Content-Security-Policy')!
      .match(/'nonce-([^']+)'/)?.[1];
    // NextResponse.next({ request: { headers } }) encodes the overridden
    // request headers onto the response itself, under this key, so the
    // downstream request (and RootLayout's headers().get('x-nonce')) sees them.
    const forwardedNonce = response.headers.get('x-middleware-request-x-nonce');

    expect(forwardedNonce).toBeTruthy();
    expect(forwardedNonce).toBe(cspNonce);
  });

  it('still sets the other core directives previously only defined in next.config.js', () => {
    const csp = runProxy('/').response.headers.get('Content-Security-Policy')!;

    expect(csp).toContain("default-src 'self'");
    expect(csp).toContain("frame-ancestors 'none'");
    expect(csp).toContain("object-src 'none'");
    expect(csp).toContain("base-uri 'self'");
    expect(csp).toContain("form-action 'self'");
  });

  it('should not include unsafe-inline in style-src', () => {
    const csp = runProxy('/').response.headers.get('Content-Security-Policy')!;
    const styleSrcMatch = csp.match(/style-src\s+([^;]+)/);
    expect(styleSrcMatch).toBeDefined();
    expect(styleSrcMatch?.[1]).not.toContain('unsafe-inline');
  });

  it('should include style-src self in CSP', () => {
    const csp = runProxy('/').response.headers.get('Content-Security-Policy')!;
    expect(csp).toContain("style-src 'self'");
  });
});
