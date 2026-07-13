import React from 'react';
import { OddsBar } from '../ui';
import { formatXLM } from '../../lib/format';

interface OutcomeOddsProps {
  label: string;
  /** Total XLM staked on this outcome. */
  pool: number;
  /** Implied probability 0-100. */
  percent: number;
  selected?: boolean;
  won?: boolean;
}

/**
 * Presentational odds row for a single outcome: label, implied percent,
 * pool size, and the shared OddsBar visualisation.
 */
export function OutcomeOdds({ label, pool, percent, selected = false, won = false }: OutcomeOddsProps) {
  const classes = [
    'outcome-odds',
    selected ? 'outcome-odds--selected' : '',
    won ? 'outcome-odds--won' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div className={classes}>
      <div className="outcome-odds__head">
        <span className="outcome-odds__label">{label}</span>
        <span className="outcome-odds__percent">{percent}%</span>
      </div>
      <OddsBar percent={percent} />
      <span className="outcome-odds__pool">{formatXLM(pool, { compact: true })} staked</span>
    </div>
  );
}
