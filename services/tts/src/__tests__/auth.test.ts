import { createHmac } from "crypto";
import {
  authenticate,
  AuthError,
  ApiKeyAuthConfig,
  JwtAuthConfig,
  TTSService,
  VOICES,
} from "../TTSService";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const API_KEY_CONFIG: ApiKeyAuthConfig = { type: "apikey", keys: ["key-abc", "key-xyz"] };
const JWT_SECRET = "test-secret";
const JWT_CONFIG: JwtAuthConfig = { type: "jwt", secret: JWT_SECRET };

function makeJwt(payload: object, secret: string): string {
  const header = Buffer.from(JSON.stringify({ alg: "HS256", typ: "JWT" })).toString("base64url");
  const body = Buffer.from(JSON.stringify(payload)).toString("base64url");
  const sig = createHmac("sha256", secret).update(`${header}.${body}`).digest("base64url");
  return `${header}.${body}.${sig}`;
}

const VALID_JWT = makeJwt({ sub: "user1", exp: Math.floor(Date.now() / 1000) + 3600 }, JWT_SECRET);
const EXPIRED_JWT = makeJwt({ sub: "user1", exp: Math.floor(Date.now() / 1000) - 60 }, JWT_SECRET);
const NO_EXP_JWT = makeJwt({ sub: "user1" }, JWT_SECRET);

// ---------------------------------------------------------------------------
// authenticate() — API key
// ---------------------------------------------------------------------------

describe("authenticate — apikey", () => {
  it("accepts a valid key", () => {
    expect(() => authenticate("key-abc", API_KEY_CONFIG)).not.toThrow();
  });

  it("throws AuthError for an invalid key", () => {
    expect(() => authenticate("bad-key", API_KEY_CONFIG)).toThrow(AuthError);
  });

  it("throws AuthError when credential is missing", () => {
    expect(() => authenticate(undefined, API_KEY_CONFIG)).toThrow(AuthError);
  });

  it("sets statusCode 401", () => {
    try {
      authenticate("bad", API_KEY_CONFIG);
    } catch (e) {
      expect((e as AuthError).statusCode).toBe(401);
    }
  });
});

// ---------------------------------------------------------------------------
// authenticate() — JWT
// ---------------------------------------------------------------------------

describe("authenticate — jwt", () => {
  it("accepts a valid JWT", () => {
    expect(() => authenticate(VALID_JWT, JWT_CONFIG)).not.toThrow();
  });

  it("throws AuthError for a wrong signature", () => {
    const tampered = VALID_JWT.slice(0, -3) + "xxx";
    expect(() => authenticate(tampered, JWT_CONFIG)).toThrow(AuthError);
  });

  it("throws AuthError for an expired JWT", () => {
    expect(() => authenticate(EXPIRED_JWT, JWT_CONFIG)).toThrow(AuthError);
  });

  it("throws AuthError for a JWT without an exp claim", () => {
    expect(() => authenticate(NO_EXP_JWT, JWT_CONFIG)).toThrow(AuthError);
  });

  it("throws AuthError for a malformed token", () => {
    expect(() => authenticate("not.a.jwt.at.all", JWT_CONFIG)).toThrow(AuthError);
  });

  it("throws AuthError when credential is missing", () => {
    expect(() => authenticate(undefined, JWT_CONFIG)).toThrow(AuthError);
  });
});

// ---------------------------------------------------------------------------
// TTSService — auth integration (no real provider calls needed)
// ---------------------------------------------------------------------------

const VOICE = VOICES["el-rachel-en"];

function makeService(authConfig?: ApiKeyAuthConfig | JwtAuthConfig) {
  return new TTSService({
    provider: "elevenlabs",
    elevenlabs: { apiKey: "el-key" },
    outputDir: "/tmp/tts-test",
    ...(authConfig ? { auth: authConfig } : {}),
  });
}

describe("TTSService.enqueue — auth", () => {
  it("allows calls when auth is not configured", () => {
    const svc = makeService();
    expect(() => svc.enqueue("hello", VOICE)).not.toThrow();
  });

  it("allows calls with a valid API key", () => {
    const svc = makeService(API_KEY_CONFIG);
    expect(() => svc.enqueue("hello", VOICE, undefined, "key-abc")).not.toThrow();
  });

  it("throws AuthError with an invalid API key", () => {
    const svc = makeService(API_KEY_CONFIG);
    expect(() => svc.enqueue("hello", VOICE, undefined, "wrong")).toThrow(AuthError);
  });

  it("throws AuthError when no credential is provided but auth is configured", () => {
    const svc = makeService(API_KEY_CONFIG);
    expect(() => svc.enqueue("hello", VOICE)).toThrow(AuthError);
  });

  it("allows calls with a valid JWT", () => {
    const svc = makeService(JWT_CONFIG);
    expect(() => svc.enqueue("hello", VOICE, undefined, VALID_JWT)).not.toThrow();
  });

  it("throws AuthError with an invalid JWT", () => {
    const svc = makeService(JWT_CONFIG);
    expect(() => svc.enqueue("hello", VOICE, undefined, "bad.jwt.token")).toThrow(AuthError);
  });
});

// ---------------------------------------------------------------------------
// TTSService — cross-tenant job isolation (#1130)
// ---------------------------------------------------------------------------

describe("TTSService — cross-tenant job isolation", () => {
  it("getJob returns undefined for a job owned by a different credential", () => {
    const svc = makeService(API_KEY_CONFIG);
    const jobId = svc.enqueue("secret narration for tenant A", VOICE, undefined, "key-abc");

    expect(svc.getJob(jobId, "key-abc")).toBeDefined();
    expect(svc.getJob(jobId, "key-xyz")).toBeUndefined();
  });

  it("listJobs only returns jobs belonging to the requesting credential", () => {
    const svc = makeService(API_KEY_CONFIG);
    const jobA = svc.enqueue("tenant A job", VOICE, undefined, "key-abc");
    const jobB = svc.enqueue("tenant B job", VOICE, undefined, "key-xyz");

    const jobsForA = svc.listJobs(undefined, "key-abc").map((j) => j.id);
    const jobsForB = svc.listJobs(undefined, "key-xyz").map((j) => j.id);

    expect(jobsForA).toContain(jobA);
    expect(jobsForA).not.toContain(jobB);
    expect(jobsForB).toContain(jobB);
    expect(jobsForB).not.toContain(jobA);
  });

  it("listAllJobsUnscoped returns jobs across every tenant", () => {
    const svc = makeService(API_KEY_CONFIG);
    const jobA = svc.enqueue("tenant A job", VOICE, undefined, "key-abc");
    const jobB = svc.enqueue("tenant B job", VOICE, undefined, "key-xyz");

    const ids = svc.listAllJobsUnscoped().map((j) => j.id);
    expect(ids).toContain(jobA);
    expect(ids).toContain(jobB);
  });
});
