import {
  listMarkets,
  getFeaturedMarkets,
  getMarket,
  createMarket,
  placeBet,
  getUserBets,
  resolveMarket,
  claimWinnings,
  resetMockData,
} from '../markets';

const USER = 'GTESTUSER0000000000000000000000000000000000000000000000000AAA';

describe('mock markets data layer', () => {
  beforeEach(() => {
    resetMockData();
  });

  it('seeds and lists markets sorted by volume', async () => {
    const markets = await listMarkets();
    expect(markets.length).toBeGreaterThan(3);
    for (let i = 1; i < markets.length; i++) {
      expect(markets[i - 1].totalVolume).toBeGreaterThanOrEqual(markets[i].totalVolume);
    }
  });

  it('filters by category, status and query', async () => {
    const crypto = await listMarkets({ category: 'Crypto' });
    expect(crypto.every((m) => m.category === 'Crypto')).toBe(true);

    const resolved = await listMarkets({ status: 'resolved' });
    expect(resolved.every((m) => m.status === 'resolved')).toBe(true);

    const byQuery = await listMarkets({ query: 'bitcoin' });
    expect(byQuery.length).toBeGreaterThan(0);
    expect(byQuery[0].title.toLowerCase()).toContain('bitcoin');
  });

  it('returns only open markets in featured, limited', async () => {
    const featured = await getFeaturedMarkets(2);
    expect(featured).toHaveLength(2);
    expect(featured.every((m) => m.status === 'open')).toBe(true);
  });

  it('gets a market by id and null for unknown', async () => {
    expect(await getMarket('mkt_btc_100k')).not.toBeNull();
    expect(await getMarket('nope')).toBeNull();
  });

  it('creates a market with zeroed pools and open status', async () => {
    const market = await createMarket({
      title: 'Test market',
      description: 'desc',
      category: 'Crypto',
      outcomes: ['Yes', 'No'],
      endsAt: new Date(Date.now() + 86_400_000).toISOString(),
      createdBy: USER,
    });
    expect(market.id).toMatch(/^mkt_/);
    expect(market.status).toBe('open');
    expect(market.totalVolume).toBe(0);
    expect(market.outcomes).toHaveLength(2);
    expect(await getMarket(market.id)).not.toBeNull();
  });

  it('places a bet and updates the pool + total volume', async () => {
    const before = await getMarket('mkt_btc_100k');
    const startPool = before!.poolByOutcome[1];
    const bet = await placeBet({ marketId: 'mkt_btc_100k', outcomeId: 1, amount: 500, user: USER });
    expect(bet.id).toMatch(/^bet_/);
    expect(bet.txHash).toBeTruthy();

    const after = await getMarket('mkt_btc_100k');
    expect(after!.poolByOutcome[1]).toBe(startPool + 500);
    expect(after!.totalVolume).toBe(before!.totalVolume + 500);

    const mine = await getUserBets(USER);
    expect(mine).toHaveLength(1);
    expect(mine[0].marketId).toBe('mkt_btc_100k');
  });

  it('rejects invalid bets', async () => {
    await expect(placeBet({ marketId: 'nope', outcomeId: 0, amount: 10, user: USER })).rejects.toThrow(
      /not found/i,
    );
    await expect(
      placeBet({ marketId: 'mkt_btc_100k', outcomeId: 1, amount: 0, user: USER }),
    ).rejects.toThrow(/greater than zero/i);
  });

  it('resolves a market and settles winners pro-rata', async () => {
    // Two users back the winning outcome; payouts split the whole pool.
    await placeBet({ marketId: 'mkt_fed_cut', outcomeId: 1, amount: 100, user: USER });
    const other = 'GOTHER';
    await placeBet({ marketId: 'mkt_fed_cut', outcomeId: 0, amount: 100, user: other });

    const resolved = await resolveMarket('mkt_fed_cut', 1);
    expect(resolved.status).toBe('resolved');
    expect(resolved.resolvedOutcome).toBe(1);

    const winnerBets = await getUserBets(USER);
    const win = winnerBets.find((b) => b.marketId === 'mkt_fed_cut')!;
    expect(win.payout).toBeGreaterThan(win.amount); // won a share of the losing pool

    const loserBets = await getUserBets(other);
    const lose = loserBets.find((b) => b.marketId === 'mkt_fed_cut')!;
    expect(lose.payout).toBe(0);
  });

  it('claims winnings once and rejects double-claim', async () => {
    await placeBet({ marketId: 'mkt_worldcup', outcomeId: 1, amount: 50, user: USER });
    await resolveMarket('mkt_worldcup', 1);
    const bet = (await getUserBets(USER)).find((b) => b.marketId === 'mkt_worldcup')!;

    const claimed = await claimWinnings(bet.id);
    expect(claimed.claimed).toBe(true);
    await expect(claimWinnings(bet.id)).rejects.toThrow(/already claimed/i);
  });

  it('cannot claim on an unresolved market', async () => {
    const bet = await placeBet({ marketId: 'mkt_xlm_1usd', outcomeId: 1, amount: 25, user: USER });
    await expect(claimWinnings(bet.id)).rejects.toThrow(/not resolved/i);
  });
});
