/**
 * Tests for GET /tts/job/:id/audio (issue #1130).
 *
 * Job responses return `outputPath`, a filesystem path local to the
 * container, but until now there was no HTTP route to actually retrieve the
 * synthesized audio. These tests exercise the new download endpoint end to
 * end: enqueue → wait for completion → download.
 */

process.env.ELEVENLABS_API_KEY = "test-audio-download-key";
process.env.TTS_OUTPUT_DIR = "/tmp/tts-audio-download-test";
delete process.env.TTS_API_KEY;

import request from "supertest";
import app from "../server";

const AUDIO_BYTES = Buffer.from("fake-mp3-bytes");

function mockSuccessfulSynthesis() {
  (global as any).fetch = jest.fn().mockResolvedValue({
    ok: true,
    arrayBuffer: async () => AUDIO_BYTES.buffer.slice(
      AUDIO_BYTES.byteOffset,
      AUDIO_BYTES.byteOffset + AUDIO_BYTES.byteLength
    ),
  });
}

async function waitForJobDone(jobId: string, timeoutMs = 5000): Promise<void> {
  const start = Date.now();
  for (;;) {
    const res = await request(app).get(`/tts/job/${jobId}`);
    if (res.body.status === "done" || res.body.status === "error") return;
    if (Date.now() - start > timeoutMs) throw new Error("Job did not complete in time");
    await new Promise((r) => setTimeout(r, 50));
  }
}

describe("GET /tts/job/:id/audio", () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it("streams the generated audio for a completed job", async () => {
    mockSuccessfulSynthesis();

    const enqueueRes = await request(app)
      .post("/tts/enqueue")
      .send({ text: "hello world", voiceId: "el-rachel-en" });
    expect(enqueueRes.status).toBe(200);
    const jobId = enqueueRes.body.jobId as string;

    await waitForJobDone(jobId);

    const jobRes = await request(app).get(`/tts/job/${jobId}`);
    expect(jobRes.body.status).toBe("done");

    const audioRes = await request(app).get(`/tts/job/${jobId}/audio`);
    expect(audioRes.status).toBe(200);
    expect(audioRes.headers["content-type"]).toMatch(/audio\/mpeg/);
    expect(Buffer.compare(audioRes.body as Buffer, AUDIO_BYTES)).toBe(0);
  });

  it("returns 404 for a non-existent job", async () => {
    const res = await request(app).get("/tts/job/does-not-exist/audio");
    expect(res.status).toBe(404);
  });

  it("returns 409 when the job has not finished processing", async () => {
    // Never resolves — job stays "processing" for the duration of this test.
    (global as any).fetch = jest.fn().mockImplementation(() => new Promise(() => {}));

    const enqueueRes = await request(app)
      .post("/tts/enqueue")
      .send({ text: "still working", voiceId: "el-rachel-en" });
    const jobId = enqueueRes.body.jobId as string;

    const audioRes = await request(app).get(`/tts/job/${jobId}/audio`);
    expect(audioRes.status).toBe(409);
  });
});

describe("GET /tts/job/:id/audio ownership enforcement", () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it("enforces the same per-owner scoping as GET /tts/job/:id", async () => {
    // Re-import server with API-key auth configured so two distinct
    // credentials produce two distinct owners.
    jest.resetModules();
    process.env.TTS_API_KEY = "key-a,key-b";
    process.env.ELEVENLABS_API_KEY = "test-audio-download-key";
    mockSuccessfulSynthesis();

    const authedApp = (await import("../server")).default;

    const enqueueRes = await request(authedApp)
      .post("/tts/enqueue")
      .set("Authorization", "Bearer key-a")
      .send({ text: "owner scoped", voiceId: "el-rachel-en" });
    expect(enqueueRes.status).toBe(200);
    const jobId = enqueueRes.body.jobId as string;

    // Poll as the owning credential until done.
    const start = Date.now();
    for (;;) {
      const jobRes = await request(authedApp)
        .get(`/tts/job/${jobId}`)
        .set("Authorization", "Bearer key-a");
      if (jobRes.body.status === "done" || jobRes.body.status === "error") break;
      if (Date.now() - start > 5000) throw new Error("Job did not complete in time");
      await new Promise((r) => setTimeout(r, 50));
    }

    // A different credential must not be able to download this job's audio.
    const otherRes = await request(authedApp)
      .get(`/tts/job/${jobId}/audio`)
      .set("Authorization", "Bearer key-b");
    expect(otherRes.status).toBe(404);

    // The owning credential can.
    const ownerRes = await request(authedApp)
      .get(`/tts/job/${jobId}/audio`)
      .set("Authorization", "Bearer key-a");
    expect(ownerRes.status).toBe(200);

    delete process.env.TTS_API_KEY;
  });
});
