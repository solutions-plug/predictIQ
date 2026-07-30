/**
 * Tests for HTTP security headers on the TTS service (fix/tts-service-sends-no-http-security-headers).
 *
 * services/tts previously served every response — including JSON error bodies
 * and job data — with only Express's bare defaults. These tests assert the
 * security headers middleware in server.ts applies to all responses.
 */

import request from "supertest";
import app from "../server";
import { TTSProviderError } from "../TTSService";
import { providerErrorMessage } from "../server";

const GENERIC_PROVIDER_ERROR = "TTS provider request failed";

describe("security headers", () => {
  it("are present on a successful response (GET /tts/voices)", async () => {
    const res = await request(app).get("/tts/voices");

    expect(res.status).toBe(200);
    expect(res.headers["x-content-type-options"]).toBe("nosniff");
    expect(res.headers["x-frame-options"]).toBe("DENY");
    expect(res.headers["referrer-policy"]).toBe("no-referrer");
    expect(res.headers["content-security-policy"]).toBeDefined();
  });

  it("are present on a health check response", async () => {
    const res = await request(app).get("/health/live");

    expect(res.headers["x-content-type-options"]).toBe("nosniff");
    expect(res.headers["x-frame-options"]).toBe("DENY");
    expect(res.headers["referrer-policy"]).toBe("no-referrer");
  });

  it("are present on a JSON error response (400)", async () => {
    const res = await request(app).post("/tts/enqueue").send({});

    expect(res.status).toBe(400);
    expect(res.headers["x-content-type-options"]).toBe("nosniff");
    expect(res.headers["x-frame-options"]).toBe("DENY");
    expect(res.headers["referrer-policy"]).toBe("no-referrer");
  });

  it("are present on a 404 response", async () => {
    const res = await request(app).get("/tts/job/does-not-exist");

    expect(res.status).toBe(404);
    expect(res.headers["x-content-type-options"]).toBe("nosniff");
    expect(res.headers["x-frame-options"]).toBe("DENY");
  });
});

describe("existing TTS functionality is unaffected by the new middleware", () => {
  it("GET /health/live still returns 200 and alive status", async () => {
    const res = await request(app).get("/health/live");
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("alive");
  });

  it("GET /tts/voices still returns the voice catalog", async () => {
    const res = await request(app).get("/tts/voices");
    expect(res.status).toBe(200);
    expect(res.body).toHaveProperty("el-rachel-en");
  });

  it("POST /tts/enqueue still enqueues a job and returns a jobId", async () => {
    const res = await request(app)
      .post("/tts/enqueue")
      .send({ text: "hello world", voiceId: "el-rachel-en" });

    expect(res.status).toBe(200);
    expect(res.body.status).toBe("pending");
    expect(typeof res.body.jobId).toBe("string");
  });

  it("POST /tts/enqueue still validates required fields", async () => {
    const res = await request(app).post("/tts/enqueue").send({ text: "hello" });
    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/Missing text or voiceId/);
  });
});

// ---------------------------------------------------------------------------
// Issue: provider error messages are sanitized (no upstream details leaked)
// ---------------------------------------------------------------------------

describe("provider error sanitization", () => {
  it("providerErrorMessage returns a generic message for TTSProviderError", () => {
    const err = new TTSProviderError("elevenlabs", "Upstream body leaked internal request-id");
    expect(providerErrorMessage(err)).toBe(GENERIC_PROVIDER_ERROR);
  });

  it("providerErrorMessage returns the original message for non-provider errors", () => {
    const err = new Error("Some other error");
    expect(providerErrorMessage(err)).toBe("Some other error");
  });
});

// ---------------------------------------------------------------------------
// Issue: provider value is validated in route handlers
// ---------------------------------------------------------------------------

describe("provider validation", () => {
  it("POST /tts/enqueue returns 400 for an unrecognized provider", async () => {
    const res = await request(app)
      .post("/tts/enqueue")
      .send({ text: "hello", voiceId: "el-rachel-en", provider: "azure" });

    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/Unknown provider/);
  });

  it("POST /tts/enqueue returns 400 for a misspelled provider", async () => {
    const res = await request(app)
      .post("/tts/enqueue")
      .send({ text: "hello", voiceId: "el-rachel-en", provider: "elevenlabs-pro" });

    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/Unknown provider/);
  });

  it("POST /tts/enqueue continues to work with provider omitted", async () => {
    const res = await request(app)
      .post("/tts/enqueue")
      .send({ text: "hello", voiceId: "el-rachel-en" });

    expect(res.status).toBe(200);
    expect(res.body.status).toBe("pending");
  });

  it("POST /tts/generate returns 400 for an unrecognized provider", async () => {
    const res = await request(app)
      .post("/tts/generate")
      .send({ text: "hello", voiceId: "el-rachel-en", provider: "unknown" });

    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/Unknown provider/);
  });

  it("POST /tts/generate continues to work with provider omitted", async () => {
    const res = await request(app)
      .post("/tts/generate")
      .send({ text: "hello", voiceId: "el-rachel-en" });

    expect(res.status).toBe(200);
    expect(res.body).toHaveProperty("outputPath");
  });
});
