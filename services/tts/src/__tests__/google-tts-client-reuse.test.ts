/**
 * Tests for perf/google-tts-client-is-re-instantiated-on-every-sy.
 *
 * generateGoogle() used to call `new TextToSpeechClient(config)` inside the
 * function body, so a brand-new client — including its credential/auth setup
 * — was constructed for every single TTS request instead of being created
 * once and reused. These tests assert the client constructor is invoked at
 * most once per distinct `google` config, regardless of how many synthesis
 * calls are made against it.
 */

import path from "path";
import os from "os";
import fs from "fs/promises";
import { TTSService, VOICES } from "../TTSService";

const mockSynthesizeSpeech = jest.fn();
const mockClientCtor = jest.fn().mockImplementation(() => ({
  synthesizeSpeech: mockSynthesizeSpeech,
}));

jest.mock("@google-cloud/text-to-speech", () => ({
  TextToSpeechClient: mockClientCtor,
}));

const VOICE = VOICES["gcp-en-us-f"];
const FAKE_MP3 = Buffer.from([0x49, 0x44, 0x33, 0xff, 0xfb, 0x90, 0x00]);

async function tmpDir(): Promise<string> {
  return fs.mkdtemp(path.join(os.tmpdir(), "tts-google-client-reuse-"));
}

describe("Google TTS client reuse (perf/google-tts-client-is-re-instantiated-on-every-sy)", () => {
  let outputDir: string;

  beforeEach(async () => {
    outputDir = await tmpDir();
    mockClientCtor.mockClear();
    mockSynthesizeSpeech.mockReset();
    mockSynthesizeSpeech.mockResolvedValue([
      { audioContent: FAKE_MP3.toString("base64") },
    ]);
  });

  it("constructs the TextToSpeechClient at most once across multiple synthesis calls", async () => {
    const svc = new TTSService({ provider: "google", google: {}, outputDir });

    await svc.generate("first request", VOICE, "google");
    await svc.generate("second request", VOICE, "google");
    await svc.generate("third request", VOICE, "google");

    expect(mockSynthesizeSpeech).toHaveBeenCalledTimes(3);
    expect(mockClientCtor).toHaveBeenCalledTimes(1);
  });

  it("reuses the same client instance for cache-bypassed repeat calls", async () => {
    const svc = new TTSService({ provider: "google", google: {}, outputDir });

    await svc.generate("hello", VOICE, "google", undefined, undefined, true);
    await svc.generate("hello", VOICE, "google", undefined, undefined, true);

    expect(mockClientCtor).toHaveBeenCalledTimes(1);
  });

  it("constructs a separate client per distinct google config (e.g. different credentials)", async () => {
    const svcA = new TTSService({
      provider: "google",
      google: { keyFilename: "a-credentials.json" },
      outputDir,
    });
    const svcB = new TTSService({
      provider: "google",
      google: { keyFilename: "b-credentials.json" },
      outputDir,
    });

    await svcA.generate("hello", VOICE, "google");
    await svcB.generate("hello", VOICE, "google");
    await svcA.generate("hello again", VOICE, "google");

    expect(mockClientCtor).toHaveBeenCalledTimes(2);
  });
});
