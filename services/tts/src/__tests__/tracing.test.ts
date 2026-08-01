/**
 * Tests for issue #1134: OpenTelemetry tracing SDK is never initialized.
 *
 * initTracing() previously existed but was never invoked anywhere, so every
 * tracer.startActiveSpan(...) call across TTSService/HealthCheck ran against
 * the default no-op tracer provider — spans were created and discarded,
 * never exported.
 */

const startMock = jest.fn();
const shutdownMock = jest.fn().mockResolvedValue(undefined);

jest.mock("@opentelemetry/sdk-node", () => ({
  NodeSDK: jest.fn().mockImplementation(() => ({
    start: startMock,
    shutdown: shutdownMock,
  })),
}));

jest.mock("@opentelemetry/exporter-trace-otlp-grpc", () => ({
  OTLPTraceExporter: jest.fn().mockImplementation(() => ({})),
}));

jest.mock("@opentelemetry/instrumentation-http", () => ({
  HttpInstrumentation: jest.fn().mockImplementation(() => ({})),
}));

describe("initTracing", () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("starts the NodeSDK", () => {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { initTracing } = require("../tracing");
    initTracing();
    expect(startMock).toHaveBeenCalledTimes(1);
  });

  it("configures the OTLP exporter and HTTP instrumentation", () => {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { NodeSDK } = require("@opentelemetry/sdk-node");
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { initTracing } = require("../tracing");
    initTracing();
    expect(NodeSDK).toHaveBeenCalledTimes(1);
    const sdkArgs = (NodeSDK as jest.Mock).mock.calls[0][0];
    expect(sdkArgs.traceExporter).toBeDefined();
    expect(sdkArgs.instrumentations).toHaveLength(1);
  });
});

describe("server startup — tracing wiring (issue #1134)", () => {
  beforeEach(() => {
    jest.resetModules();
    process.env.TTS_OUTPUT_DIR = "/tmp/tts-tracing-test";
  });

  it("calls initTracing() exactly once at module load, before request handling", () => {
    const initTracingMock = jest.fn();
    jest.doMock("../tracing", () => ({ initTracing: initTracingMock }));

    // require.main !== module in Jest, so server.ts's app.listen() is skipped —
    // importing it here only exercises the module's top-level wiring.
    require("../server");

    expect(initTracingMock).toHaveBeenCalledTimes(1);
  });
});
