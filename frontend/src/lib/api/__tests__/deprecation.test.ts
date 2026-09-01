import {
  reportResponseHeaders,
  currentDeprecation,
  onDeprecation,
  _resetDeprecationForTests,
} from '../deprecation';

function headers(init: Record<string, string>): Headers {
  const h = new Headers();
  for (const [k, v] of Object.entries(init)) h.set(k, v);
  return h;
}

describe('deprecation signal bus (#1337)', () => {
  beforeEach(() => _resetDeprecationForTests());
  afterEach(() => _resetDeprecationForTests());

  it('is a no-op for a response with no Deprecation header', () => {
    const listener = jest.fn();
    onDeprecation(listener);

    reportResponseHeaders(headers({ 'Content-Type': 'application/json' }));

    expect(listener).not.toHaveBeenCalled();
    expect(currentDeprecation()).toBeNull();
  });

  it('treats Deprecation: false as not deprecated (no false positive)', () => {
    reportResponseHeaders(headers({ Deprecation: 'false' }));
    expect(currentDeprecation()).toBeNull();
  });

  it('opens a signal with the sunset date and parsed migration link', () => {
    const listener = jest.fn();
    onDeprecation(listener);

    reportResponseHeaders(
      headers({
        Deprecation: 'true',
        Sunset: 'Wed, 01 Jul 2026 00:00:00 GMT',
        Link: '<https://docs.predictiq.dev/api/migrate-v2>; rel="deprecation"',
      }),
    );

    expect(listener).toHaveBeenCalledWith({
      sunset: 'Wed, 01 Jul 2026 00:00:00 GMT',
      migrationUrl: 'https://docs.predictiq.dev/api/migrate-v2',
    });
    expect(currentDeprecation()?.migrationUrl).toBe('https://docs.predictiq.dev/api/migrate-v2');
  });

  it('a later non-deprecated response does not clear an active signal', () => {
    reportResponseHeaders(headers({ Deprecation: 'true', Sunset: '2026-07-01' }));
    reportResponseHeaders(headers({ 'Content-Type': 'application/json' }));
    expect(currentDeprecation()?.sunset).toBe('2026-07-01');
  });

  it('falls back to a bare Link url when no matching rel is present', () => {
    reportResponseHeaders(
      headers({ Deprecation: 'true', Link: '<https://example.com/notes>' }),
    );
    expect(currentDeprecation()?.migrationUrl).toBe('https://example.com/notes');
  });
});
