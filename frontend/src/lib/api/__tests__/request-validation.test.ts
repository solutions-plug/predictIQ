import { api, ApiError } from '../public-client';
import { apiCache } from '../cache';
import {
  newsletterSubscribeSchema,
  emailRequestSchema,
  placeBetSchema,
  __contract,
} from '../requestSchemas';

/**
 * #1341 — outgoing request bodies are validated against the generated-schema
 * contract before the request is sent, so a malformed body fails a local
 * assertion instead of producing a raw 400 from the server.
 */

describe('request body schemas', () => {
  it('module-level contract holds (z.infer matches components[schemas])', () => {
    // `__contract` only type-checks when every schema still matches its
    // generated type; this asserts the runtime value too.
    expect(__contract).toEqual([
      [true, true],
      [true, true],
      [true, true],
    ]);
  });

  it('accepts a body with only the required fields', () => {
    expect(newsletterSubscribeSchema.safeParse({ email: 'a@b.com' }).success).toBe(true);
  });

  it('does not reject a legitimately-optional field being present or absent', () => {
    expect(newsletterSubscribeSchema.safeParse({ email: 'a@b.com', source: 'footer' }).success).toBe(true);
    expect(newsletterSubscribeSchema.safeParse({ email: 'a@b.com' }).success).toBe(true);
  });

  it('does not reject an unknown extra field (loose mode / forward-compat)', () => {
    expect(newsletterSubscribeSchema.safeParse({ email: 'a@b.com', utm: 'x' }).success).toBe(true);
    expect(emailRequestSchema.safeParse({ email: 'a@b.com', note: 'x' }).success).toBe(true);
  });

  it('rejects a malformed email', () => {
    expect(newsletterSubscribeSchema.safeParse({ email: 'not-an-email' }).success).toBe(false);
  });

  it('rejects an invalid placeBet body', () => {
    expect(placeBetSchema.safeParse({ wallet: 'GABC', outcome: -1, amount: '10' }).success).toBe(false);
    expect(placeBetSchema.safeParse({ wallet: 'GABC', outcome: 1, amount: 'lots' }).success).toBe(false);
    expect(placeBetSchema.safeParse({ wallet: 'GABC', outcome: 1, amount: '10.5' }).success).toBe(true);
  });
});

describe('boundary validation in the client', () => {
  const originalFetch = global.fetch;
  const originalEnv = process.env.NEXT_PUBLIC_API_URL;

  beforeEach(() => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    apiCache.clear();
    // A server that would answer every request with a raw 400.
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 400,
      statusText: 'Bad Request',
      json: async () => ({ message: 'validation failed' }),
    });
  });

  afterEach(() => {
    jest.restoreAllMocks();
    process.env.NEXT_PUBLIC_API_URL = originalEnv;
    global.fetch = originalFetch;
  });

  it('rejects a malformed newsletterSubscribe body locally, without calling fetch', async () => {
    await expect(
      api.newsletterSubscribe({ email: 'nope' }),
    ).rejects.toMatchObject({ code: 'CLIENT_VALIDATION_ERROR' });

    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('rejects a malformed placeBet body locally, without calling fetch', async () => {
    await expect(
      api.placeBet(1, { wallet: 'GABC', outcome: -3, amount: '10' }),
    ).rejects.toBeInstanceOf(ApiError);

    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('sends a well-formed body through to the network', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ success: true, message: 'ok' }),
    });

    await api.newsletterSubscribe({ email: 'valid@example.com', source: 'hero' });
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it('surfaces the failing field in the error message', async () => {
    await expect(
      api.newsletterSubscribe({ email: 'nope' }),
    ).rejects.toThrow(/email/);
  });
});
