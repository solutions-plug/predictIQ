/**
 * Test for issue #1132: rate limiter bypass via a rotating Authorization header.
 *
 * The bucket key must be derived from the caller's IP address only — never
 * from the (unvalidated, optional) Authorization header — otherwise a caller
 * can rotate a fake credential on every request to get a fresh rate-limit
 * bucket each time.
 */

import { Request } from "express";
import rateLimit from "express-rate-limit";
import { rateLimitKeyGenerator } from "../rateLimitKey";

function mockRequest(ip: string, authHeader?: string): Request {
  return {
    ip,
    headers: authHeader ? { authorization: authHeader } : {},
  } as unknown as Request;
}

describe("rateLimitKeyGenerator", () => {
  it("keys on IP regardless of Authorization header presence", () => {
    const withAuth = mockRequest("1.2.3.4", "Bearer fake-token");
    const withoutAuth = mockRequest("1.2.3.4");
    expect(rateLimitKeyGenerator(withAuth)).toBe(rateLimitKeyGenerator(withoutAuth));
  });

  it("returns the same key across a rotating fake Authorization header", () => {
    const keys = new Set<string>();
    for (let i = 0; i < 20; i++) {
      keys.add(rateLimitKeyGenerator(mockRequest("9.8.7.6", `Bearer fake-token-${i}`)));
    }
    // Every request should collapse to the same bucket key.
    expect(keys.size).toBe(1);
    expect([...keys][0]).toBe("ip:9.8.7.6");
  });

  it("isolates buckets by IP", () => {
    expect(rateLimitKeyGenerator(mockRequest("1.1.1.1"))).not.toBe(
      rateLimitKeyGenerator(mockRequest("2.2.2.2"))
    );
  });
});

describe("Express rate limiter — rotating Authorization header", () => {
  it("still enforces the limit when the caller rotates a fake Authorization header per request", async () => {
    const limiter = rateLimit({
      windowMs: 60_000,
      max: 3,
      keyGenerator: rateLimitKeyGenerator,
      standardHeaders: false,
      legacyHeaders: false,
    });

    const ip = "5.5.5.5";
    let blocked = 0;
    let allowed = 0;

    for (let i = 0; i < 6; i++) {
      const req = mockRequest(ip, `Bearer rotating-fake-${i}`);
      const res: any = {
        setHeader: jest.fn(),
        getHeader: jest.fn(),
        removeHeader: jest.fn(),
        status: jest.fn().mockReturnThis(),
        json: jest.fn(),
        send: jest.fn(),
        end: jest.fn(),
        headersSent: false,
      };
      let nextCalled = false;
      await new Promise<void>((resolve) => {
        limiter(req, res, () => {
          nextCalled = true;
          resolve();
        });
        // express-rate-limit's handler path doesn't call next(); resolve for that case too.
        if (res.status.mock.calls.length > 0) resolve();
      });
      if (nextCalled) allowed++;
      else blocked++;
    }

    expect(allowed).toBe(3);
    expect(blocked).toBe(3);
  });
});
