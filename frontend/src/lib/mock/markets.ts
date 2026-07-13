/**
 * Mock data layer for the PredictIQ app.
 *
 * This is the ONLY data source for the app surface right now — every feature
 * reads and writes through these async functions (they mimic the shape of a
 * real API so wiring the backend later is a drop-in swap). State is persisted
 * to localStorage so bets/markets survive reloads within a browser session.
 */

import type {
  Bet,
  CreateMarketInput,
  Market,
  PlaceBetInput,
} from '../types';

const STORAGE_KEY = 'predictiq:mock:v1';
const NET_DELAY = 320; // ms — lets the UI show real loading states

interface Db {
  markets: Market[];
  bets: Bet[];
}

function iso(daysFromNow: number): string {
  return new Date(Date.now() + daysFromNow * 86_400_000).toISOString();
}

function seed(): Db {
  const markets: Market[] = [
    {
      id: 'mkt_btc_100k',
      title: 'Will Bitcoin close above $100k in 2026?',
      description:
        'Resolves YES if the BTC/USD daily close is at or above $100,000 on any day before the market end date, per the Pyth oracle feed.',
      category: 'Crypto',
      outcomes: [
        { id: 1, label: 'Yes' },
        { id: 0, label: 'No' },
      ],
      poolByOutcome: { 1: 18400, 0: 7600 },
      totalVolume: 26000,
      endsAt: iso(45),
      status: 'open',
      resolvedOutcome: null,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000AAA',
      createdAt: iso(-20),
    },
    {
      id: 'mkt_xlm_1usd',
      title: 'Will Stellar (XLM) reach $1.00 before Q3 2026?',
      description:
        'Resolves YES if XLM/USD trades at or above $1.00 on a major exchange before the end date.',
      category: 'Crypto',
      outcomes: [
        { id: 1, label: 'Yes' },
        { id: 0, label: 'No' },
      ],
      poolByOutcome: { 1: 5200, 0: 9800 },
      totalVolume: 15000,
      endsAt: iso(70),
      status: 'open',
      resolvedOutcome: null,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000BBB',
      createdAt: iso(-12),
    },
    {
      id: 'mkt_election',
      title: 'Which party wins the 2026 general election?',
      description:
        'Resolves to the party that secures a governing majority, per the certified national result.',
      category: 'Politics',
      outcomes: [
        { id: 0, label: 'Progressive' },
        { id: 1, label: 'Conservative' },
        { id: 2, label: 'Independent' },
      ],
      poolByOutcome: { 0: 12000, 1: 14500, 2: 3500 },
      totalVolume: 30000,
      endsAt: iso(120),
      status: 'open',
      resolvedOutcome: null,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000CCC',
      createdAt: iso(-30),
    },
    {
      id: 'mkt_worldcup',
      title: 'Will the host nation reach the World Cup semi-finals?',
      description: 'Resolves YES if the host national team advances to the semi-final stage.',
      category: 'Sports',
      outcomes: [
        { id: 1, label: 'Yes' },
        { id: 0, label: 'No' },
      ],
      poolByOutcome: { 1: 8800, 0: 8200 },
      totalVolume: 17000,
      endsAt: iso(15),
      status: 'open',
      resolvedOutcome: null,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000DDD',
      createdAt: iso(-8),
    },
    {
      id: 'mkt_ai_agi',
      title: 'Will a major lab announce AGI before 2027?',
      description:
        'Resolves YES if a top-5 AI lab publicly claims to have achieved AGI, corroborated by two independent outlets.',
      category: 'Technology',
      outcomes: [
        { id: 1, label: 'Yes' },
        { id: 0, label: 'No' },
      ],
      poolByOutcome: { 1: 4200, 0: 21800 },
      totalVolume: 26000,
      endsAt: iso(200),
      status: 'open',
      resolvedOutcome: null,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000EEE',
      createdAt: iso(-5),
    },
    {
      id: 'mkt_fed_cut',
      title: 'Will the Fed cut rates at the next meeting?',
      description: 'Resolves YES if the target range is lowered at the next scheduled FOMC meeting.',
      category: 'Economics',
      outcomes: [
        { id: 1, label: 'Yes' },
        { id: 0, label: 'No' },
      ],
      poolByOutcome: { 1: 16000, 0: 4000 },
      totalVolume: 20000,
      endsAt: iso(9),
      status: 'open',
      resolvedOutcome: null,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000FFF',
      createdAt: iso(-3),
    },
    {
      id: 'mkt_resolved_demo',
      title: 'Did the 2025 protocol upgrade ship on time?',
      description: 'A resolved market, kept for demonstrating settled/claimable states.',
      category: 'Technology',
      outcomes: [
        { id: 1, label: 'Yes' },
        { id: 0, label: 'No' },
      ],
      poolByOutcome: { 1: 9000, 0: 3000 },
      totalVolume: 12000,
      endsAt: iso(-2),
      status: 'resolved',
      resolvedOutcome: 1,
      createdBy: 'GBSEED000000000000000000000000000000000000000000000000000GGG',
      createdAt: iso(-40),
    },
  ];

  return { markets, bets: [] };
}

function load(): Db {
  if (typeof window === 'undefined') return seed();
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as Db;
  } catch {
    /* fall through to seed */
  }
  const fresh = seed();
  save(fresh);
  return fresh;
}

function save(db: Db): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(db));
  } catch {
    /* ignore quota / privacy-mode errors */
  }
}

function delay<T>(value: T): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), NET_DELAY));
}

function uid(prefix: string): string {
  return `${prefix}_${Math.random().toString(36).slice(2, 10)}`;
}

function recalc(market: Market): Market {
  const totalVolume = Object.values(market.poolByOutcome).reduce((a, b) => a + b, 0);
  return { ...market, totalVolume };
}

// ---------------------------------------------------------------------------
// Public async API (mirrors a real REST client)
// ---------------------------------------------------------------------------

export async function listMarkets(filters?: {
  category?: string;
  status?: Market['status'];
  query?: string;
}): Promise<Market[]> {
  const { markets } = load();
  let result = [...markets].sort((a, b) => b.totalVolume - a.totalVolume);
  if (filters?.category) result = result.filter((m) => m.category === filters.category);
  if (filters?.status) result = result.filter((m) => m.status === filters.status);
  if (filters?.query) {
    const q = filters.query.toLowerCase();
    result = result.filter(
      (m) => m.title.toLowerCase().includes(q) || m.description.toLowerCase().includes(q),
    );
  }
  return delay(result);
}

export async function getFeaturedMarkets(limit = 3): Promise<Market[]> {
  const { markets } = load();
  const open = markets.filter((m) => m.status === 'open').sort((a, b) => b.totalVolume - a.totalVolume);
  return delay(open.slice(0, limit));
}

export async function getMarket(id: string): Promise<Market | null> {
  const { markets } = load();
  return delay(markets.find((m) => m.id === id) ?? null);
}

export async function createMarket(input: CreateMarketInput): Promise<Market> {
  const db = load();
  const outcomes = input.outcomes.map((label, i) => ({ id: i, label }));
  const market: Market = {
    id: uid('mkt'),
    title: input.title,
    description: input.description,
    category: input.category,
    outcomes,
    poolByOutcome: Object.fromEntries(outcomes.map((o) => [o.id, 0])),
    totalVolume: 0,
    endsAt: input.endsAt,
    status: 'open',
    resolvedOutcome: null,
    createdBy: input.createdBy,
    createdAt: new Date().toISOString(),
  };
  db.markets.unshift(market);
  save(db);
  return delay(market);
}

export async function placeBet(input: PlaceBetInput): Promise<Bet> {
  const db = load();
  const market = db.markets.find((m) => m.id === input.marketId);
  if (!market) throw new Error('Market not found');
  if (market.status !== 'open') throw new Error('This market is no longer open');
  if (input.amount <= 0) throw new Error('Stake must be greater than zero');

  market.poolByOutcome[input.outcomeId] =
    (market.poolByOutcome[input.outcomeId] ?? 0) + input.amount;
  Object.assign(market, recalc(market));

  const bet: Bet = {
    id: uid('bet'),
    marketId: input.marketId,
    outcomeId: input.outcomeId,
    amount: input.amount,
    user: input.user,
    placedAt: new Date().toISOString(),
    claimed: false,
    payout: null,
    txHash: input.txHash ?? uid('tx'),
  };
  db.bets.unshift(bet);
  save(db);
  return delay(bet);
}

export async function getUserBets(user: string): Promise<Bet[]> {
  const { bets } = load();
  return delay(bets.filter((b) => b.user === user).sort((a, b) => b.placedAt.localeCompare(a.placedAt)));
}

export async function resolveMarket(id: string, outcomeId: number): Promise<Market> {
  const db = load();
  const market = db.markets.find((m) => m.id === id);
  if (!market) throw new Error('Market not found');
  market.status = 'resolved';
  market.resolvedOutcome = outcomeId;

  // Settle every bet on this market: winners split the whole pool pro-rata.
  const winningPool = market.poolByOutcome[outcomeId] ?? 0;
  for (const bet of db.bets.filter((b) => b.marketId === id)) {
    if (bet.outcomeId === outcomeId && winningPool > 0) {
      bet.payout = (bet.amount / winningPool) * market.totalVolume;
    } else {
      bet.payout = 0;
    }
  }
  save(db);
  return delay(market);
}

export async function claimWinnings(betId: string): Promise<Bet> {
  const db = load();
  const bet = db.bets.find((b) => b.id === betId);
  if (!bet) throw new Error('Bet not found');
  const market = db.markets.find((m) => m.id === bet.marketId);
  if (!market || market.status !== 'resolved') throw new Error('Market is not resolved yet');
  if (bet.claimed) throw new Error('Already claimed');
  bet.claimed = true;
  save(db);
  return delay(bet);
}

/** Test/util hook: wipe persisted state back to the seed. */
export function resetMockData(): void {
  if (typeof window !== 'undefined') window.localStorage.removeItem(STORAGE_KEY);
}
