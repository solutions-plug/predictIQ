import { buildAuthConfig } from "../authConfig";

describe("buildAuthConfig", () => {
  it("returns undefined outside production when unset", () => {
    expect(buildAuthConfig({})).toBeUndefined();
  });
  it("throws in production when unset", () => {
    expect(() => buildAuthConfig({ NODE_ENV: "production" })).toThrow();
  });
  it("throws when the key list is empty after trimming", () => {
    expect(() => buildAuthConfig({ TTS_API_KEY: " , " })).toThrow();
  });
  it("trims and drops empty entries", () => {
    expect(buildAuthConfig({ TTS_API_KEY: "a, b,," })).toEqual({
      type: "apikey",
      keys: ["a", "b"],
    });
  });
});

describe("buildAuthConfig error messages and isolation", () => {
  it("gives a descriptive error when the key is missing in production", () => {
    expect(() => buildAuthConfig({ NODE_ENV: "production" })).toThrow(
      "TTS_API_KEY must be set when NODE_ENV=production"
    );
  });
  it("treats an empty string as missing in production", () => {
    expect(() => buildAuthConfig({ NODE_ENV: "production", TTS_API_KEY: "" })).toThrow(
      "TTS_API_KEY must be set"
    );
  });
  it.each([",", " , ,", "   "])("rejects malformed value %j with a clear message", (v) => {
    expect(() => buildAuthConfig({ TTS_API_KEY: v })).toThrow(
      "TTS_API_KEY is set but contains no usable keys"
    );
  });
  it("resolves a valid production config to the expected object", () => {
    expect(buildAuthConfig({ NODE_ENV: "production", TTS_API_KEY: "k1,k2" })).toEqual({
      type: "apikey",
      keys: ["k1", "k2"],
    });
  });
  it("does not read process.env when an env object is supplied", () => {
    const prev = process.env.TTS_API_KEY;
    process.env.TTS_API_KEY = "leak";
    try {
      expect(buildAuthConfig({})).toBeUndefined();
    } finally {
      if (prev === undefined) delete process.env.TTS_API_KEY;
      else process.env.TTS_API_KEY = prev;
    }
  });
});
