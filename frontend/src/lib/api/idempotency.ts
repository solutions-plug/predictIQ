/**
 * Idempotency-key generation for mutating API requests (#1340).
 *
 * The backend's idempotency layer (services/api/src/idempotency.rs, exposed in
 * services/api/openapi.yaml as the `Idempotency-Key` header) deduplicates a
 * mutating request when the same key is seen twice inside the idempotency
 * window. Without the client attaching a key, a network-layer retry of a bet
 * placement or market creation could create a duplicate.
 *
 * Contract:
 *   - One key represents one *logical submission*. It is generated once, at the
 *     call site, and reused across every automatic retry of that submission
 *     (the retry loop in public-client.ts / admin-client.ts keeps the header
 *     fixed for the lifetime of a single `request()` call).
 *   - A genuinely new user-initiated submission (e.g. the user edits a form
 *     after a failure and submits again) is a new `request()` call, so it gets
 *     a fresh key.
 */

// openapi.yaml caps the header at 128 chars; a UUID v4 is 36, well within it.
const MAX_KEY_LENGTH = 128;

/**
 * Returns a fresh idempotency key (UUID v4 when the platform provides one).
 *
 * `crypto.randomUUID` is available in every browser the app targets and in
 * Node >= 19; the manual fallback keeps unit tests and older runtimes working
 * without pulling in a uuid dependency.
 */
export function newIdempotencyKey(): string {
  const cryptoObj = typeof globalThis !== 'undefined' ? globalThis.crypto : undefined;

  if (cryptoObj && typeof cryptoObj.randomUUID === 'function') {
    return cryptoObj.randomUUID();
  }

  if (cryptoObj && typeof cryptoObj.getRandomValues === 'function') {
    const bytes = cryptoObj.getRandomValues(new Uint8Array(16));
    // RFC 4122 version + variant bits.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  }

  // Last-resort fallback: still unique enough to deduplicate retries within a
  // window, which is all this key needs to do.
  return `idmp-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`;
}

/** True when `value` is a usable idempotency key for the `Idempotency-Key` header. */
export function isValidIdempotencyKey(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_KEY_LENGTH;
}
