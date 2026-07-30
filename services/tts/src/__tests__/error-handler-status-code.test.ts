/**
 * Tests for fix/global-error-handler-always-returns-http-500-dis.
 *
 * The final Express error-handling middleware used to unconditionally
 * respond 500, ignoring err.status/err.statusCode entirely — so a malformed
 * JSON request body (a SyntaxError thrown inside express.json(), uncaught by
 * any route-level try/catch) was reported to the client as a 500 rather than
 * a 400.
 */

import request from "supertest";
import app, { globalErrorHandler } from "../server";
import { Response } from "express";

function mockResponse(): Response {
  const res: any = {};
  res.status = jest.fn().mockReturnValue(res);
  res.json = jest.fn().mockReturnValue(res);
  return res as Response;
}

describe("globalErrorHandler (unit)", () => {
  it("defaults to 500 with a generic message when the error carries no statusCode/status", () => {
    const res = mockResponse();
    globalErrorHandler(new Error("some internal detail"), {} as any, res, jest.fn());

    expect(res.status).toHaveBeenCalledWith(500);
    expect(res.json).toHaveBeenCalledWith({ error: "Internal server error" });
  });

  it("respects err.statusCode when present", () => {
    const res = mockResponse();
    const err: any = new Error("missing field foo");
    err.statusCode = 400;
    globalErrorHandler(err, {} as any, res, jest.fn());

    expect(res.status).toHaveBeenCalledWith(400);
    expect(res.json).toHaveBeenCalledWith({ error: "missing field foo" });
  });

  it("respects err.status when statusCode is absent (e.g. body-parser SyntaxError shape)", () => {
    const res = mockResponse();
    const err: any = new SyntaxError("Unexpected token o in JSON at position 1");
    err.status = 400;
    globalErrorHandler(err, {} as any, res, jest.fn());

    expect(res.status).toHaveBeenCalledWith(400);
    expect(res.json).toHaveBeenCalledWith({ error: err.message });
  });

  it("keeps a generic message for 5xx even when explicitly set", () => {
    const res = mockResponse();
    const err: any = new Error("upstream connection reset");
    err.statusCode = 503;
    globalErrorHandler(err, {} as any, res, jest.fn());

    expect(res.status).toHaveBeenCalledWith(503);
    expect(res.json).toHaveBeenCalledWith({ error: "Internal server error" });
  });
});

describe("malformed JSON request body (integration)", () => {
  it("POST /tts/enqueue with malformed JSON returns 400, not 500", async () => {
    const res = await request(app)
      .post("/tts/enqueue")
      .set("Content-Type", "application/json")
      .send("{ this is not valid json");

    expect(res.status).toBe(400);
    expect(res.body.error).toBeDefined();
  });

  it("POST /tts/generate with malformed JSON returns 400, not 500", async () => {
    const res = await request(app)
      .post("/tts/generate")
      .set("Content-Type", "application/json")
      .send("{ also not valid json");

    expect(res.status).toBe(400);
    expect(res.body.error).toBeDefined();
  });
});
