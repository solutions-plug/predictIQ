/**
 * API deprecation signal bus (#1337).
 *
 * `API_SPEC.md`'s deprecation policy: deprecated versions return `Deprecation`,
 * `Sunset`, and `Link` response headers with a 12-month minimum support window.
 * The client reports every response's headers here; the UI subscribes to show a
 * dismissible banner.
 *
 * False-positive guard: a response with no `Deprecation` header (or `Deprecation:
 * false`) is a no-op - it never clears a real signal, but it also never opens one,
 * so a single stale/cached non-deprecated response cannot suppress or fabricate the
 * banner.
 */

export interface DeprecationInfo {
  /** ISO date (or HTTP-date) the version stops being supported, or null if unknown. */
  sunset: string | null;
  /** URL of the migration guide, parsed from the `Link` header, or null. */
  migrationUrl: string | null;
}

type Listener = (info: DeprecationInfo) => void;

const listeners = new Set<Listener>();
let current: DeprecationInfo | null = null;

export function onDeprecation(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The most recent deprecation signal, or null if the API has not reported one. */
export function currentDeprecation(): DeprecationInfo | null {
  return current;
}

/** `<https://docs.example/migrate>; rel="deprecation"` -> `https://docs.example/migrate` */
function parseMigrationLink(linkHeader: string | null): string | null {
  if (!linkHeader) return null;
  for (const part of linkHeader.split(',')) {
    const url = part.match(/<([^>]+)>/)?.[1];
    const rel = part.match(/rel="?([^";]+)"?/)?.[1];
    if (url && (rel === 'deprecation' || rel === 'sunset' || rel === 'successor-version')) {
      return url;
    }
  }
  // A single bare `<url>` with no rel is still better than nothing.
  return linkHeader.match(/<([^>]+)>/)?.[1] ?? null;
}

/**
 * Feed one response's headers in. Acts only when `Deprecation` is present and not
 * `false`; otherwise a no-op.
 */
export function reportResponseHeaders(headers: Headers | null | undefined): void {
  if (!headers || typeof headers.get !== 'function') return;
  const deprecation = headers.get('Deprecation');
  if (!deprecation || deprecation.toLowerCase() === 'false') return;

  const info: DeprecationInfo = {
    sunset: headers.get('Sunset'),
    migrationUrl: parseMigrationLink(headers.get('Link')),
  };
  current = info;
  for (const listener of listeners) listener(info);
}

/** Test helper. */
export function _resetDeprecationForTests(): void {
  current = null;
  listeners.clear();
}
