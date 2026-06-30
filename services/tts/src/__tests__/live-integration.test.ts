/**
 * Live integration tests for TTS providers (issue #978).
 *
 * These tests make REAL API calls and consume quota/credits.
 * They are gated by the INTEGRATION_TEST=true environment variable so they
 * never run in normal CI — only in dedicated integration test jobs or locally
 * when a developer explicitly opts in.
 *
 * Run:
 *   INTEGRATION_TEST=true \
 *   ELEVENLABS_API_KEY=<key> \
 *   GOOGLE_APPLICATION_CREDENTIALS=<path> \
 *   jest live-integration
 */

import path from "path";
import os from "os";
import fs from "fs/promises";
import { TTSService, VOICES, type TTSVoice } from "../TTSService";

// Skip the entire suite unless explicitly opted in.
const RUN = process.env["INTEGRATION_TEST"] === "true";

const describeIf = (cond: boolean) => (cond ? describe : describe.skip);

async function tmpDir(): Promise<string> {
  return fs.mkdtemp(path.join(os.tmpdir(), "tts-live-"));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * Validate that `filePath` points to a non-empty file whose first bytes look
 * like an MP3 (ID3v2 header or MPEG sync word).
 */
async function assertLooksLikeMp3(filePath: string): Promise<void> {
  const stat = await fs.stat(filePath);
  expect(stat.size).toBeGreaterThan(0);

  const fd = await fs.open(filePath, "r");
  const headerBuf = Buffer.alloc(4);
  await fd.read(headerBuf, 0, 4, 0);
  await fd.close();

  const isId3 = headerBuf[0] === 0x49 && headerBuf[1] === 0x44 && headerBuf[2] === 0x33;
  const isMpegSync = headerBuf[0] === 0xff && (headerBuf[1] & 0xe0) === 0xe0;

  expect(isId3 || isMpegSync).toBe(true);
}

// ── ElevenLabs live tests ─────────────────────────────────────────────────────

describeIf(RUN)("ElevenLabs — live integration", () => {
  const apiKey = process.env["ELEVENLABS_API_KEY"];

  beforeAll(() => {
    if (!apiKey) {
      throw new Error(
        "ELEVENLABS_API_KEY must be set to run ElevenLabs live integration tests"
      );
    }
  });

  it("synthesizes speech and returns a valid MP3 file", async () => {
    const outputDir = await tmpDir();
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: apiKey! },
      outputDir,
    });

    const voice: TTSVoice = VOICES["el-rachel-en"];
    const outPath = await svc.generate(
      "Hello from the PredictIQ live integration test.",
      voice,
      "elevenlabs"
    );

    await assertLooksLikeMp3(outPath);
  }, 30_000);

  it("respects voice_settings in the request body (stability / similarity_boost)", async () => {
    // This test verifies the request schema sent to the real API does not
    // trigger a validation error (which would cause a non-2xx response).
    const outputDir = await tmpDir();
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: apiKey!, modelId: "eleven_multilingual_v2" },
      outputDir,
    });

    const voice: TTSVoice = VOICES["el-adam-en"];
    const outPath = await svc.generate("Testing voice settings.", voice, "elevenlabs");
    await assertLooksLikeMp3(outPath);
  }, 30_000);

  it("rejects a clearly invalid API key with TTSProviderError", async () => {
    const outputDir = await tmpDir();
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "invalid-key-for-testing" },
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICES["el-rachel-en"], "elevenlabs")
    ).rejects.toMatchObject({ name: "TTSProviderError", provider: "elevenlabs" });
  }, 15_000);
});

// ── Google Cloud TTS live tests ───────────────────────────────────────────────

describeIf(RUN)("Google Cloud TTS — live integration", () => {
  const credentialsPath = process.env["GOOGLE_APPLICATION_CREDENTIALS"];

  beforeAll(() => {
    if (!credentialsPath) {
      throw new Error(
        "GOOGLE_APPLICATION_CREDENTIALS must be set to run Google TTS live integration tests"
      );
    }
  });

  it("synthesizes speech and returns a valid MP3 file", async () => {
    const outputDir = await tmpDir();
    const svc = new TTSService({
      provider: "google",
      google: { keyFilename: credentialsPath },
      outputDir,
    });

    const voice: TTSVoice = VOICES["gcp-en-us-f"];
    const outPath = await svc.generate(
      "Hello from the PredictIQ live integration test.",
      voice,
      "google"
    );

    await assertLooksLikeMp3(outPath);
  }, 30_000);

  it("synthesizes non-English speech (es-ES)", async () => {
    const outputDir = await tmpDir();
    const svc = new TTSService({
      provider: "google",
      google: { keyFilename: credentialsPath },
      outputDir,
    });

    const voice: TTSVoice = VOICES["gcp-es-es-f"];
    const outPath = await svc.generate(
      "Hola desde la prueba de integración en vivo.",
      voice,
      "google"
    );

    await assertLooksLikeMp3(outPath);
  }, 30_000);

  it("rejects bad credentials with TTSProviderError", async () => {
    const outputDir = await tmpDir();
    const svc = new TTSService({
      provider: "google",
      // Pass an empty credentials object — the SDK will reject it.
      google: { credentials: {} },
      outputDir,
    });

    await expect(
      svc.generate("Hello", VOICES["gcp-en-us-f"], "google")
    ).rejects.toMatchObject({ name: "TTSProviderError", provider: "google" });
  }, 15_000);
});

// ── Cross-provider fallback live test ─────────────────────────────────────────

describeIf(RUN && !!process.env["ELEVENLABS_API_KEY"] && !!process.env["GOOGLE_APPLICATION_CREDENTIALS"])(
  "Provider fallback — live integration",
  () => {
    it("falls back to Google when ElevenLabs is given a bad key", async () => {
      const outputDir = await tmpDir();
      const svc = new TTSService({
        provider: "elevenlabs",
        elevenlabs: { apiKey: "invalid-key" },
        google: { keyFilename: process.env["GOOGLE_APPLICATION_CREDENTIALS"] },
        outputDir,
      });

      const outPath = await svc.generate(
        "Fallback test.",
        VOICES["gcp-en-us-f"],
        "elevenlabs"
      );

      await assertLooksLikeMp3(outPath);
    }, 30_000);
  }
);
