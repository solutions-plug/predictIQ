import { apiCache, CACHE_TTL } from '../cache';

describe('apiCache.dedupe (#1334)', () => {
  beforeEach(() => apiCache.clear());

  it('shares one in-flight promise for concurrent calls with the same key', async () => {
    const factory = jest.fn(
      () => new Promise<string>((resolve) => setTimeout(() => resolve('value'), 10)),
    );

    const [a, b, c] = await Promise.all([
      apiCache.dedupe('k', factory),
      apiCache.dedupe('k', factory),
      apiCache.dedupe('k', factory),
    ]);

    expect(factory).toHaveBeenCalledTimes(1);
    expect([a, b, c]).toEqual(['value', 'value', 'value']);
  });

  it('forgets the in-flight promise once it settles, so a later call re-runs', async () => {
    const factory = jest.fn(async () => 'v');
    await apiCache.dedupe('k', factory);
    expect(apiCache.inFlightCount).toBe(0);
    await apiCache.dedupe('k', factory);
    expect(factory).toHaveBeenCalledTimes(2);
  });

  it('does not de-dupe different keys', async () => {
    const factory = jest.fn(async (v: string) => v);
    await Promise.all([
      apiCache.dedupe('a', () => factory('a')),
      apiCache.dedupe('b', () => factory('b')),
    ]);
    expect(factory).toHaveBeenCalledTimes(2);
  });

  it('forgets a rejected in-flight promise too', async () => {
    const factory = jest
      .fn()
      .mockRejectedValueOnce(new Error('boom'))
      .mockResolvedValueOnce('ok');

    await expect(apiCache.dedupe('k', factory)).rejects.toThrow('boom');
    expect(apiCache.inFlightCount).toBe(0);
    await expect(apiCache.dedupe('k', factory)).resolves.toBe('ok');
  });
});

describe('apiCache.invalidateTag (#1334)', () => {
  beforeEach(() => apiCache.clear());

  it('drops only entries carrying the tag', () => {
    apiCache.set('m1', { id: 1 }, CACHE_TTL.SHORT, ['market:1']);
    apiCache.set('stats', { n: 5 }, CACHE_TTL.SHORT, ['statistics']);

    apiCache.invalidateTag('market:1');

    expect(apiCache.get('m1')).toBeNull();
    expect(apiCache.get('stats')).toEqual({ n: 5 });
  });
});
