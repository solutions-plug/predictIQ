import { EventEmitter } from "events";

jest.mock("../tracing", () => ({ shutdownTracing: jest.fn(() => Promise.resolve()) }));

import { registerGracefulShutdown } from "../gracefulShutdown";

describe("registerGracefulShutdown", () => {
  let exitSpy: jest.SpyInstance;
  let onSpy: jest.SpyInstance;
  let handlers: Record<string, () => void>;

  beforeEach(() => {
    jest.useFakeTimers();
    handlers = {};
    exitSpy = jest.spyOn(process, "exit").mockImplementation((() => undefined) as any);
    onSpy = jest.spyOn(process, "on").mockImplementation(((evt: string, fn: () => void) => {
      handlers[evt] = fn;
      return process;
    }) as any);
    jest.spyOn(console, "log").mockImplementation(() => undefined);
    jest.spyOn(console, "error").mockImplementation(() => undefined);
  });

  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  function fakeServer() {
    const emitter = new EventEmitter();
    let closeCb: (() => void) | undefined;
    return {
      close: jest.fn((cb: () => void) => { closeCb = cb; }),
      closeIdleConnections: jest.fn(),
      finishInFlight: () => closeCb?.(),
      emitter,
    };
  }

  it("lets the in-flight request finish: exit(0) only after server.close completes, no forced exit", async () => {
    const server = fakeServer();
    registerGracefulShutdown(server as any, 1000);
    handlers.SIGTERM();

    expect(server.close).toHaveBeenCalled();
    expect(exitSpy).not.toHaveBeenCalled(); // request still in flight

    jest.advanceTimersByTime(500); // within the timeout
    expect(exitSpy).not.toHaveBeenCalled();

    server.finishInFlight(); // in-flight request completed
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    expect(exitSpy).toHaveBeenCalledWith(0);
    expect(exitSpy).not.toHaveBeenCalledWith(1);
  });

  it("forces exit(1) when a request never completes within the timeout", () => {
    const server = fakeServer();
    registerGracefulShutdown(server as any, 1000);
    handlers.SIGINT();

    expect(exitSpy).not.toHaveBeenCalled();
    jest.advanceTimersByTime(1000);
    expect(exitSpy).toHaveBeenCalledWith(1);
  });

  it("ignores a second signal while already shutting down", () => {
    const server = fakeServer();
    registerGracefulShutdown(server as any, 1000);
    handlers.SIGTERM();
    handlers.SIGTERM();
    expect(server.close).toHaveBeenCalledTimes(1);
  });
});
