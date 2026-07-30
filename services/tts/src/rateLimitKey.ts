/**
 * Rate-limit bucket key derivation for the Express-level limiter (issue #995).
 *
 * Split into its own module so it can be unit-tested without booting the
 * full server (issue #1132).
 */

import { Request } from "express";

/**
 * Key the limiter on the caller's IP address only. The Authorization header
 * MUST NOT be used here: it is unvalidated at this point in the middleware
 * chain (auth runs after rate limiting, and is optional entirely), so a
 * caller could otherwise send a different fake credential on every request
 * to get a fresh bucket each time, bypassing the throttle.
 */
export function rateLimitKeyGenerator(req: Request): string {
  return `ip:${req.ip ?? "unknown"}`;
}
