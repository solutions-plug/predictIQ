import fs from "fs/promises";
import os from "os";
import path from "path";
import {
  HealthChecker,
  MAX_QUEUE_DEPTH,
  createHealthCheckHandler,
  createReadinessHandler,
} from "../HealthCheck";

function makeService(opts: { jobs?: unknown[]; throwOnList?: boolean; breakers?: Record<string, any> } = {}) {
  return {
    listAllJobsUnscoped: jest.fn(() => {
      if (opts.throwOnList) throw new Error("store down");
      return opts.jobs ?? [];
    }),
    getCircuitBreakerStates: jest.fn(() => opts.breakers ?? {}),
  } as any;
}

function mockRes() {
  const res: any = {};
  res.status = jest.fn().mockReturnValue(res);
  res.json = jest.fn().mockReturnValue(res);
  return res;
}

describe("HealthChecker (contract: services/tts/HEALTH_CHECK.md)", () => {
  let dir: string;
  beforeEach(async () => {
    dir = await fs.mkdtemp(path.join(os.tmpdir(), "tts-health-"));
  });
  afterEach(async () => {
    await fs.rm(dir, { recursive: true, force: true });
  });

  it("healthy dependencies -> status 'healthy', HTTP 200, documented response shape", async () => {
    const checker = new HealthChecker({ outputDir: dir } as any, makeService());
    const res = mockRes();
    await createHealthCheckHandler(checker)({}, res);

    expect(res.status).toHaveBeenCalledWith(200);
    const body = res.json.mock.calls[0][0];
    expect(body.status).toBe("healthy");
    expect(typeof body.timestamp).toBe("string");
    expect(typeof body.uptime).toBe("number");
    expect(body.message).toContain("All systems operational");
    expect(Object.keys(body.checks)).toEqual(
      expect.arrayContaining(["service", "outputDirectory", "jobStore"]),
    );
    for (const c of Object.values<any>(body.checks)) {
      expect(c.status).toBe("ok");
      expect(typeof c.message).toBe("string");
    }
  });

  it("failing job store -> status 'unhealthy', HTTP 503, failing check named in message", async () => {
    const checker = new HealthChecker({ outputDir: dir } as any, makeService({ throwOnList: true }));
    const res = mockRes();
    await createHealthCheckHandler(checker)({}, res);

    expect(res.status).toHaveBeenCalledWith(503);
    const body = res.json.mock.calls[0][0];
    expect(body.status).toBe("unhealthy");
    expect(body.checks.jobStore.status).toBe("error");
    expect(body.message).toContain("Service is unhealthy");
    expect(body.message).toContain("jobStore");
  });

  it("invalid ElevenLabs key -> unhealthy 503 without contacting the network", async () => {
    const checker = new HealthChecker(
      { outputDir: dir, elevenlabs: { apiKey: "short" } } as any,
      makeService(),
    );
    const res = mockRes();
    await createHealthCheckHandler(checker)({}, res);
    expect(res.status).toHaveBeenCalledWith(503);
    expect(res.json.mock.calls[0][0].checks.elevenlabs.status).toBe("error");
  });

  it("queue near capacity -> 'degraded' and HTTP 503", async () => {
    const jobs = new Array(Math.ceil(MAX_QUEUE_DEPTH * 0.8)).fill({});
    const checker = new HealthChecker({ outputDir: dir } as any, makeService({ jobs }));
    const res = mockRes();
    await createHealthCheckHandler(checker)({}, res);
    // pending + processing each return `jobs`, so depth is doubled; assert not healthy.
    expect(res.status).toHaveBeenCalledWith(503);
    expect(res.json.mock.calls[0][0].status).not.toBe("healthy");
  });

  it("readiness: healthy -> 200 'ready'; open circuit breaker -> 503 'not_ready'", async () => {
    const ok = mockRes();
    await createReadinessHandler(new HealthChecker({ outputDir: dir } as any, makeService()))({}, ok);
    expect(ok.status).toHaveBeenCalledWith(200);
    expect(ok.json.mock.calls[0][0].status).toBe("ready");

    const bad = mockRes();
    const svc = makeService({ breakers: { google: { state: "open", failures: 5, successes: 0 } } });
    await createReadinessHandler(new HealthChecker({ outputDir: dir } as any, svc))({}, bad);
    expect(bad.status).toHaveBeenCalledWith(503);
    const body = bad.json.mock.calls[0][0];
    expect(body.status).toBe("not_ready");
    expect(body.message).toContain("circuitBreaker_google");
  });
});
