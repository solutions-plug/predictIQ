import nextConfig from '../../next.config';

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

  it('should have Content-Security-Policy header configured', () => {
    const header = headers.find((h: any) => h.key === 'Content-Security-Policy');
    expect(header).toBeDefined();
    expect(header.value).toContain("default-src 'self'");
    expect(header.value).toContain("script-src 'self'");
    expect(header.value).toContain("style-src 'self'");
    expect(header.value).toContain("frame-ancestors 'none'");
  });

  it('should have all required security headers', () => {
    const requiredHeaders = [
      'X-Content-Type-Options',
      'X-Frame-Options',
      'Referrer-Policy',
      'Permissions-Policy',
      'Content-Security-Policy',
    ];

    const headerKeys = headers.map((h: any) => h.key);
    requiredHeaders.forEach(headerKey => {
      expect(headerKeys).toContain(headerKey);
    });
  });
});
