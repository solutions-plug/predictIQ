/**
 * Contract tests for TTS provider integrations (issue #978).
 *
 * Uses pre-recorded (fixture) responses — the VCR pattern — so tests run
 * offline in CI without real API keys.  Each fixture represents the canonical
 * response schema documented by the provider; if a provider changes its schema
 * the fixture diverges and the test fails before production does.
 *
 * Run: jest provider-contracts
 */

import path from "path";
import os from "os";
import fs from "fs/promises";
import {
  TTSService,
  VOICES,
  type TTSVoice,
} from "../TTSService";

// ── Google TTS mock — hoisted by jest so it intercepts the dynamic require()
//    inside generateGoogle at runtime.
// ─────────────────────────────────────────────────────────────────────────────

const mockSynthesizeSpeech = jest.fn();

jest.mock("@google-cloud/text-to-speech", () => ({
  TextToSpeechClient: jest.fn().mockImplementation(() => ({
    synthesizeSpeech: mockSynthesizeSpeech,
  })),
}));

// ── Fixture data ──────────────────────────────────────────────────────────────

// A minimal valid MP3 frame (ID3v2 header + silent MPEG frame).
const FAKE_MP3 = Buffer.from([
  0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ID3v2 header
  0xff, 0xfb, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // MPEG frame
]);

// Recorded ElevenLabs synthesize response —
//   POST /v1/text-to-speech/{voice_id}
//   200 Content-Type: audio/mpeg
const ELEVENLABS_FIXTURE = {
  status: 200,
  headers: { "content-type": "audio/mpeg" },
  body: FAKE_MP3,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

function mockFetchSuccess(): jest.SpyInstance {
  return jest.spyOn(global, "fetch").mockResolvedValue(
    new Response(ELEVENLABS_FIXTURE.body, {
      status: ELEVENLABS_FIXTURE.status,
      headers: ELEVENLABS_FIXTURE.headers,
    })
  );
}

function mockFetchError(status: number, body = "Error"): jest.SpyInstance {
  return jest.spyOn(global, "fetch").mockResolvedValue(
    new Response(body, { status })
  );
}

function mockFetchNetworkError(): jest.SpyInstance {
  return jest
    .spyOn(global, "fetch")
    .mockRejectedValue(new TypeError("Failed to fetch"));
}

async function tmpDir(): Promise<string> {
  return fs.mkdtemp(path.join(os.tmpdir(), "tts-contract-"));
}

const VOICE_EL: TTSVoice = VOICES["el-rachel-en"];
const VOICE_GCP: TTSVoice = VOICES["gcp-en-us-f"];

// ── ElevenLabs contract ───────────────────────────────────────────────────────

describe("ElevenLabs provider — contract (VCR)", () => {
  let fetchSpy: jest.SpyInstance;
  let outputDir: string;

  beforeEach(async () => {
    outputDir = await tmpDir();
    fetchSpy = mockFetchSuccess();
  });

  afterEach(() => {
    fetchSpy.mockRestore();
  });

  it("calls the correct endpoint URL for a given voice ID", async () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    await svc.generate("Hello world", VOICE_EL, "elevenlabs");

    const [url] = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect(url).toContain(`/v1/text-to-speech/${VOICE_EL.voiceId}`);
    expect(url).toContain("api.elevenlabs.io");
  });

  it("sends the xi-api-key header", async () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "el-secret-key" },
      outputDir,
    });

    await svc.generate("Hello", VOICE_EL, "elevenlabs");

    const [, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    const headers = init.headers as Record<string, string>;
    expect(headers["xi-api-key"]).toBe("el-secret-key");
  });

  it("sends Accept: audio/mpeg", async () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    await svc.generate("Hello", VOICE_EL, "elevenlabs");

    const [, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    const headers = init.headers as Record<string, string>;
    expect(headers["Accept"]).toBe("audio/mpeg");
  });

  it("includes text and model_id in the request body", async () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key", modelId: "eleven_monolingual_v1" },
      outputDir,
    });

    await svc.generate("Contract test", VOICE_EL, "elevenlabs");

    const [, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    const body = JSON.parse(init.body as string) as Record<string, unknown>;
    expect(body["text"]).toBe("Contract test");
    expect(body["model_id"]).toBe("eleven_monolingual_v1");
  });

  it("defaults to eleven_multilingual_v2 when modelId is omitted", async () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    await svc.generate("Hello", VOICE_EL, "elevenlabs");

    const [, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    const body = JSON.parse(init.body as string) as Record<string, unknown>;
    expect(body["model_id"]).toBe("eleven_multilingual_v2");
  });

  it("writes the recorded audio body to disk", async () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    const outPath = await svc.generate("Hello", VOICE_EL, "elevenlabs");
    const written = await fs.readFile(outPath);
    expect(written).toEqual(FAKE_MP3);
  });

  it("surfaces an error containing provider name on HTTP 401 (auth failure schema)", async () => {
    fetchSpy.mockRestore();
    fetchSpy = mockFetchError(401, "Unauthorized");

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "bad-key" },
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICE_EL, "elevenlabs")
    ).rejects.toMatchObject({ message: expect.stringContaining("elevenlabs") });
  });

  it("surfaces an error containing provider name on HTTP 429 (rate limit schema)", async () => {
    fetchSpy.mockRestore();
    fetchSpy = mockFetchError(429, "Too Many Requests");

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICE_EL, "elevenlabs")
    ).rejects.toMatchObject({ message: expect.stringContaining("ElevenLabs") });
  });

  it("surfaces an error on HTTP 500 (server error schema)", async () => {
    fetchSpy.mockRestore();
    fetchSpy = mockFetchError(500, "Internal Server Error");

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICE_EL, "elevenlabs")
    ).rejects.toThrow();
  });

  it("surfaces an error on network failure (no response schema)", async () => {
    fetchSpy.mockRestore();
    fetchSpy = mockFetchNetworkError();

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICE_EL, "elevenlabs")
    ).rejects.toThrow();
  });
});

// ── Google Cloud TTS contract ─────────────────────────────────────────────────

describe("Google Cloud TTS provider — contract (VCR)", () => {
  let outputDir: string;

  beforeEach(async () => {
    outputDir = await tmpDir();
    // Reset the mock before each test so behaviour can be configured per-test.
    mockSynthesizeSpeech.mockReset();
    mockSynthesizeSpeech.mockResolvedValue([
      { audioContent: FAKE_MP3.toString("base64") },
    ]);
  });

  it("calls synthesizeSpeech with the correct voice and language", async () => {
    const svc = new TTSService({ provider: "google", google: {}, outputDir });

    await svc.generate("Hello world", VOICE_GCP, "google");

    expect(mockSynthesizeSpeech).toHaveBeenCalledTimes(1);
    const [req] = mockSynthesizeSpeech.mock.calls[0] as [
      { voice: { name: string; languageCode: string }; audioConfig: { audioEncoding: string } }
    ];
    expect(req.voice.name).toBe(VOICE_GCP.voiceId);
    expect(req.voice.languageCode).toBe(VOICE_GCP.language);
    expect(req.audioConfig.audioEncoding).toBe("MP3");
  });

  it("writes the decoded audioContent buffer to disk", async () => {
    const svc = new TTSService({ provider: "google", google: {}, outputDir });

    const outPath = await svc.generate("Hello", VOICE_GCP, "google");
    const written = await fs.readFile(outPath);
    expect(written).toEqual(FAKE_MP3);
  });

  it("handles base64-encoded audioContent (string variant from SDK)", async () => {
    // Some SDK versions return a base64 string instead of a Buffer.
    mockSynthesizeSpeech.mockResolvedValue([
      { audioContent: FAKE_MP3.toString("base64") },
    ]);

    const svc = new TTSService({ provider: "google", google: {}, outputDir });
    const outPath = await svc.generate("Hello", VOICE_GCP, "google");
    const written = await fs.readFile(outPath);
    expect(written).toEqual(FAKE_MP3);
  });

  it("surfaces an error containing provider name on SDK auth failure schema", async () => {
    mockSynthesizeSpeech.mockRejectedValue(
      new Error("UNAUTHENTICATED: Request had invalid credentials.")
    );

    const svc = new TTSService({ provider: "google", google: {}, outputDir });

    await expect(
      svc.generate("Hello", VOICE_GCP, "google")
    ).rejects.toMatchObject({ message: expect.stringContaining("google") });
  });

  it("surfaces an error containing provider name on quota exceeded schema", async () => {
    mockSynthesizeSpeech.mockRejectedValue(
      new Error("RESOURCE_EXHAUSTED: Quota exceeded")
    );

    const svc = new TTSService({ provider: "google", google: {}, outputDir });

    await expect(
      svc.generate("Hello", VOICE_GCP, "google")
    ).rejects.toMatchObject({ message: expect.stringContaining("google") });
  });
});

// ── Fallback contract ─────────────────────────────────────────────────────────

describe("provider fallback contract", () => {
  let fetchSpy: jest.SpyInstance;
  let outputDir: string;

  beforeEach(async () => {
    outputDir = await tmpDir();
    mockSynthesizeSpeech.mockReset();
  });

  afterEach(() => {
    fetchSpy?.mockRestore();
  });

  it("falls back to Google when ElevenLabs returns 503 (recorded fixture)", async () => {
    // ElevenLabs returns 503
    fetchSpy = mockFetchError(503, "Service Unavailable");
    // Google mock returns success
    mockSynthesizeSpeech.mockResolvedValue([
      { audioContent: FAKE_MP3.toString("base64") },
    ]);

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      google: {},
      outputDir,
    });

    const outPath = await svc.generate("Hello", VOICE_GCP, "elevenlabs");
    const written = await fs.readFile(outPath);
    expect(written).toEqual(FAKE_MP3);
  });

  it("throws when both providers fail (both recorded as errors)", async () => {
    fetchSpy = mockFetchError(503, "Service Unavailable");
    mockSynthesizeSpeech.mockRejectedValue(new Error("UNAUTHENTICATED"));

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      google: {},
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICE_GCP, "elevenlabs")
    ).rejects.toThrow();
  });
});
