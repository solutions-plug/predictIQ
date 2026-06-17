import { buildCspHeader } from '../middleware';

describe('buildCspHeader', () => {
  const TEST_NONCE = 'dGVzdG5vbmNl';

  it('includes the nonce in script-src', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).toContain(`'nonce-${TEST_NONCE}'`);
  });

  it('includes strict-dynamic in script-src', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).toContain("'strict-dynamic'");
  });

  it('includes upgrade-insecure-requests', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).toContain('upgrade-insecure-requests');
  });

  it('blocks frame embedding via frame-ancestors none', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).toContain("frame-ancestors 'none'");
  });

  it('restricts form targets to same origin', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).toContain("form-action 'self'");
  });

  it('restricts base URI to same origin', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).toContain("base-uri 'self'");
  });

  it('does not contain unsafe-eval', () => {
    const csp = buildCspHeader(TEST_NONCE);
    expect(csp).not.toContain("'unsafe-eval'");
  });

  it('does not contain unsafe-inline in script-src', () => {
    const csp = buildCspHeader(TEST_NONCE);
    const scriptSrc = csp.split(';').find(d => d.trim().startsWith('script-src'));
    expect(scriptSrc).not.toContain("'unsafe-inline'");
  });

  it('produces different CSP values for different nonces', () => {
    const csp1 = buildCspHeader('nonce-one');
    const csp2 = buildCspHeader('nonce-two');
    expect(csp1).not.toBe(csp2);
  });
});
