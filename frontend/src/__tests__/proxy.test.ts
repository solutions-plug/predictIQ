jest.mock('next/server', () => ({
  NextResponse: {
    next: jest.fn(() => ({
      headers: new Map(),
    })),
  },
  NextRequest: jest.fn(),
}));

import { proxy } from '../proxy';

describe('Proxy CSP', () => {
  const originalCrypto = global.crypto;

  beforeAll(() => {
    (global as any).crypto = {
      randomUUID: jest.fn(() => 'test-nonce-uuid'),
    };
  });

  afterAll(() => {
    (global as any).crypto = originalCrypto;
  });

  function makeRequest(headers: Record<string, string> = {}) {
    return {
      headers: new Headers(headers),
    };
  }

  it('should not include unsafe-inline in style-src', () => {
    const headers = new Map<string, string>();
    const mockNextResponse = {
      headers: {
        set: (key: string, value: string) => headers.set(key, value),
        get: (key: string) => headers.get(key),
      },
    };

    jest.requireMock('next/server').NextResponse.next.mockReturnValue(mockNextResponse as any);

    const request = makeRequest();
    proxy(request as any);

    const csp = headers.get('Content-Security-Policy') ?? '';
    const styleSrcMatch = csp.match(/style-src\s+([^;]+)/);
    expect(styleSrcMatch).toBeDefined();
    expect(styleSrcMatch?.[1]).not.toContain('unsafe-inline');
  });

  it('should include style-src self in CSP', () => {
    const headers = new Map<string, string>();
    const mockNextResponse = {
      headers: {
        set: (key: string, value: string) => headers.set(key, value),
        get: (key: string) => headers.get(key),
      },
    };

    jest.requireMock('next/server').NextResponse.next.mockReturnValue(mockNextResponse as any);

    const request = makeRequest();
    proxy(request as any);

    const csp = headers.get('Content-Security-Policy') ?? '';
    expect(csp).toContain("style-src 'self'");
  });
});