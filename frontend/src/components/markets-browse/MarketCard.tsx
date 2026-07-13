import Link from 'next/link';
import type { Market } from '../../lib/types';
import { formatXLM, timeUntil, outcomeOdds } from '../../lib/format';
import { Card, Badge, StatusBadge, OddsBar } from '../ui';

/** Pick the outcome with the largest implied probability. */
function topOutcome(market: Market) {
  return market.outcomes.reduce((best, o) =>
    outcomeOdds(market, o.id) > outcomeOdds(market, best.id) ? o : best,
  );
}

export function MarketCard({ market, featured = false }: { market: Market; featured?: boolean }) {
  const lead = topOutcome(market);
  const odds = outcomeOdds(market, lead.id);
  const ended = market.status !== 'open';

  return (
    <Link href={`/markets/${market.id}`} className="market-card-link">
      <Card
        as="article"
        interactive
        className={`market-card${featured ? ' market-card--featured' : ''}`}
      >
        <header className="market-card__head">
          <Badge tone={featured ? 'gold' : 'default'}>{market.category}</Badge>
          <StatusBadge status={market.status} />
        </header>

        <h3 className="market-card__title">{market.title}</h3>

        <div className="market-card__odds">
          <div className="market-card__odds-row">
            <span className="market-card__outcome">{lead.label}</span>
            <span className="market-card__pct">{odds}%</span>
          </div>
          <OddsBar percent={odds} />
        </div>

        <footer className="market-card__foot">
          <span className="market-card__vol">{formatXLM(market.totalVolume, { compact: true })}</span>
          <span className="market-card__time">{ended ? 'Ended' : timeUntil(market.endsAt)}</span>
        </footer>
      </Card>
    </Link>
  );
}
