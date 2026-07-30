/**
 * OpenTelemetry tracing initialization for TTS service
 */

import { NodeSDK } from "@opentelemetry/sdk-node";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-grpc";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { ATTR_SERVICE_NAME, ATTR_SERVICE_VERSION } from "@opentelemetry/semantic-conventions";
import { HttpInstrumentation } from "@opentelemetry/instrumentation-http";

export function initTracing() {
  const otlpEndpoint = process.env.OTEL_EXPORTER_OTLP_ENDPOINT || "http://localhost:4317";
  const serviceName = process.env.OTEL_SERVICE_NAME || "predictiq-tts";
  const samplingRatio = parseFloat(process.env.OTEL_TRACE_SAMPLING_RATIO || "1.0");

  const traceExporter = new OTLPTraceExporter({
    url: otlpEndpoint,
  });

  const sdk = new NodeSDK({
    resource: resourceFromAttributes({
      [ATTR_SERVICE_NAME]: serviceName,
      [ATTR_SERVICE_VERSION]: "1.0.0",
    }),
    traceExporter,
    instrumentations: [new HttpInstrumentation()],
  });

  sdk.start();

  // Graceful shutdown
  process.on("SIGTERM", () => {
    sdk
      .shutdown()
      .then(() => console.log("Tracing terminated"))
      .catch((error) => console.error("Error terminating tracing", error))
      .finally(() => process.exit(0));
  });

  return sdk;
}
