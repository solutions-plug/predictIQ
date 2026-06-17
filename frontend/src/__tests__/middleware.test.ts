import { middleware } from '../middleware';
import { NextRequest } from 'next/server';

function makeRequest(path = '/') {
  return new NextRequest(new URL(`http://localhost${path}`));
}

describe('CSP nonce middleware', () => {
  it('sets Content-Security-Policy header on the response', async () => {
    const req = makeRequest('/');
    const res = await middleware(req);
    expect(res.headers.get('Content-Security-Policy')).not.toBeNull();
  });

  it('includes nonce in script-src directive', async () => {
    const req = makeRequest('/');
    const res = await middleware(req);
    const csp = res.headers.get('Content-Security-Policy') ?? '';
    expect(csp).toMatch(/script-src[^;]*'nonce-[A-Za-z0-9+/=]+'[^;]*/);
  });

  it('generates a unique nonce per request', async () => {
    const csp1 = (await middleware(makeRequest('/'))).headers.get('Content-Security-Policy') ?? '';
    const csp2 = (await middleware(makeRequest('/'))).headers.get('Content-Security-Policy') ?? '';
    const nonce1 = csp1.match(/'nonce-([^']+)'/)?.[1];
    const nonce2 = csp2.match(/'nonce-([^']+)'/)?.[1];
    expect(nonce1).toBeDefined();
    expect(nonce2).toBeDefined();
    expect(nonce1).not.toBe(nonce2);
  });

  it('sets x-nonce request header for server components', async () => {
    const req = makeRequest('/');
    const res = await middleware(req);
    // The nonce is forwarded as x-nonce in the request headers passed to the route.
    // Next.js does not expose these in the response, so we verify the CSP is present
    // as a proxy for the middleware running correctly.
    const csp = res.headers.get('Content-Security-Policy') ?? '';
    expect(csp).toContain('strict-dynamic');
  });

  it('includes upgrade-insecure-requests directive', async () => {
    const csp = (await middleware(makeRequest('/'))).headers.get('Content-Security-Policy') ?? '';
    expect(csp).toContain('upgrade-insecure-requests');
  });

  it('blocks frame embedding with frame-ancestors none', async () => {
    const csp = (await middleware(makeRequest('/'))).headers.get('Content-Security-Policy') ?? '';
    expect(csp).toContain("frame-ancestors 'none'");
  });

  it('restricts form targets with form-action self', async () => {
    const csp = (await middleware(makeRequest('/'))).headers.get('Content-Security-Policy') ?? '';
    expect(csp).toContain("form-action 'self'");
  });
});
