/**
 * Tests for fix/no-trust-proxy-configuration-makes-req-ip-based.
 *
 * req.ip is used both as the Express-level rate-limit key and as the
 * rateLimitKey passed into TTSService. Without app.set("trust proxy", ...),
 * Express derives req.ip from the raw socket address — deployed behind any
 * reverse proxy/load balancer, every request then appears to originate from
 * the proxy's address, collapsing all distinct clients into a single
 * rate-limit bucket. These tests assert TRUST_PROXY is honored so req.ip
 * reflects X-Forwarded-For under a configured proxy topology.
 *
 * Each test re-requires ../server after setting TRUST_PROXY, since the
 * setting is applied once at module load time via app.set(...).
 */

describe("trust proxy configuration", () => {
  const ORIGINAL_ENV = { ...process.env };

  afterEach(() => {
    process.env = { ...ORIGINAL_ENV };
    jest.resetModules();
  });

  it("defaults to trusting 1 hop, so req.ip reflects X-Forwarded-For behind a single load balancer", async () => {
    jest.resetModules();
    delete process.env.TRUST_PROXY;

    const request = require("supertest");
    const app = require("../server").default;
    app.get("/__whoami", (req: any, res: any) => res.json({ ip: req.ip }));

    const res = await request(app)
      .get("/__whoami")
      .set("X-Forwarded-For", "203.0.113.7");

    expect(res.body.ip).toBe("203.0.113.7");
  });

  it("ignores X-Forwarded-For when TRUST_PROXY=false (no proxy in front)", async () => {
    jest.resetModules();
    process.env.TRUST_PROXY = "false";

    const request = require("supertest");
    const app = require("../server").default;
    app.get("/__whoami", (req: any, res: any) => res.json({ ip: req.ip }));

    const res = await request(app)
      .get("/__whoami")
      .set("X-Forwarded-For", "203.0.113.7");

    expect(res.body.ip).not.toBe("203.0.113.7");
  });

  it("distinguishes different clients behind a configured reverse proxy via X-Forwarded-For", async () => {
    jest.resetModules();
    process.env.TRUST_PROXY = "1";

    const request = require("supertest");
    const app = require("../server").default;
    app.get("/__whoami", (req: any, res: any) => res.json({ ip: req.ip }));

    const resA = await request(app).get("/__whoami").set("X-Forwarded-For", "198.51.100.1");
    const resB = await request(app).get("/__whoami").set("X-Forwarded-For", "198.51.100.2");

    expect(resA.body.ip).toBe("198.51.100.1");
    expect(resB.body.ip).toBe("198.51.100.2");
    expect(resA.body.ip).not.toBe(resB.body.ip);
  });

  it("honors an explicit numeric hop count greater than 1", async () => {
    jest.resetModules();
    process.env.TRUST_PROXY = "2";

    const request = require("supertest");
    const app = require("../server").default;
    app.get("/__whoami", (req: any, res: any) => res.json({ ip: req.ip }));

    // Two hops: X-Forwarded-For "client, proxy1" — with trust proxy = 2,
    // Express counts back 2 hops from the socket peer, landing on "client".
    const res = await request(app)
      .get("/__whoami")
      .set("X-Forwarded-For", "203.0.113.9, 10.0.0.1");

    expect(res.body.ip).toBe("203.0.113.9");
  });
});
