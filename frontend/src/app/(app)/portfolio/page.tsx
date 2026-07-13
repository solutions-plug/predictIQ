import type { Metadata } from 'next';
import { PortfolioView } from '../../../components/portfolio/PortfolioView';

export const metadata: Metadata = {
  title: 'Portfolio · PredictIQ',
  description: 'Track your open positions, settled bets, and claimable winnings.',
};

export default function PortfolioPage() {
  return <PortfolioView />;
}
