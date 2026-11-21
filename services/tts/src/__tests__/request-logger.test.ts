import { EventEmitter } from "events";
import { requestLogger } from "../requestLogger";

function run(req: any) {
  const res: any = new EventEmitter();
  res.statusCode = 201;
  res.setHeader = jest.fn();
  const next = jest.fn();
  requestLogger(req, res, next);
  res.emit("finish");
  expect(next).toHaveBeenCalled();
}

describe("requestLogger", () => {
  let lines: string[];
  beforeEach(() => {
    lines = [];
    jest.spyOn(console, "log").mockImplementation((l: string) => { lines.push(l); });
  });
  afterEach(() => jest.restoreAllMocks());

  it("never emits authorization or cookie values in the serialized log record", () => {
    run({
      method: "POST",
      path: "/tts/synthesize",
      headers: { authorization: "Bearer SECRET-TOKEN", cookie: "sid=SECRET-COOKIE" },
    });
    expect(lines).toHaveLength(1);
    expect(lines[0]).not.toContain("SECRET-TOKEN");
    expect(lines[0]).not.toContain("SECRET-COOKIE");
    expect(lines[0].toLowerCase()).not.toContain("authorization");
    expect(lines[0].toLowerCase()).not.toContain("cookie");
  });

  it("preserves method, path and status in the log record (job ids redacted)", () => {
    run({ method: "GET", path: "/tts/job/abc-123", headers: {} });
    const rec = JSON.parse(lines[0]);
    expect(rec.method).toBe("GET");
    expect(rec.path).toBe("/tts/job/:id");
    expect(rec.status).toBe(201);
    expect(rec.msg).toBe("request");
  });

  it("handles requests with no headers at all", () => {
    run({ method: "GET", path: "/health", headers: {} });
    const rec = JSON.parse(lines[0]);
    expect(rec.requestId).toEqual(expect.any(String));
    expect(rec.traceId).toBeUndefined();
  });
});
