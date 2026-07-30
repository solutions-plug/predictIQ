import http from "http";
import { AddressInfo } from "net";
import app from "../server";

describe("security headers", () => {
  it("adds standard security headers to responses", async () => {
    const server = app.listen(0, "127.0.0.1");

    await new Promise<void>((resolve, reject) => {
      server.once("listening", () => resolve());
      server.once("error", reject);
    });

    const { port } = server.address() as AddressInfo;

    try {
      const response = await new Promise<http.IncomingMessage>((resolve, reject) => {
        const req = http.get({ hostname: "127.0.0.1", port, path: "/health" }, (res) => {
          resolve(res);
        });
        req.on("error", reject);
      });

      const chunks: Buffer[] = [];
      for await (const chunk of response) {
        chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
      }

      expect(response.headers["x-content-type-options"]).toBe("nosniff");
      expect(response.headers["x-frame-options"]).toBe("DENY");
      expect(response.headers["referrer-policy"]).toBe("no-referrer");
      expect(response.headers["content-security-policy"]).toContain("default-src 'none'");
    } finally {
      await new Promise<void>((resolve, reject) => {
        server.close((err) => (err ? reject(err) : resolve()));
      });
    }
  });
});
