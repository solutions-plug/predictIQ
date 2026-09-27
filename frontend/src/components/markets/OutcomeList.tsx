import React from 'react';
import './OutcomeList.css';

export interface MarketOutcome {
  id: string;
  label: string;
  /** Probability weight, 0-1. */
  odds: number;
}

interface OutcomeListProps {
  outcomes: MarketOutcome[];
  /** Outcome count above which the list becomes a scrollable region. Defaults to 6. */
  scrollThreshold?: number;
}

const formatOdds = (odds: number): string => `${(odds * 100).toFixed(1)}%`;

export const OutcomeList: React.FC<OutcomeListProps> = ({ outcomes, scrollThreshold = 6 }) => {
  if (!outcomes || outcomes.length === 0) {
    return <div className="outcome-list outcome-list--empty">No outcomes available</div>;
  }

  const scrollable = outcomes.length > scrollThreshold;

  return (
    <ul
      className={`outcome-list${scrollable ? ' outcome-list--scrollable' : ''}`}
      aria-label={`Market outcomes (${outcomes.length})`}
    >
      {outcomes.map((outcome) => (
        <li key={outcome.id} className="outcome-list__item">
          <span className="outcome-list__label">{outcome.label}</span>
          <span className="outcome-list__odds">{formatOdds(outcome.odds)}</span>
        </li>
      ))}
    </ul>
  );
};

export default OutcomeList;
