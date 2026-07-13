import type { Metadata } from 'next';
import { MarketsExplorer } from '../../../components/markets-browse/MarketsExplorer';

export const metadata: Metadata = {
  title: 'Markets · PredictIQ',
  description: 'Browse open and resolved prediction markets on PredictIQ.',
};

export default function MarketsPage() {
  return <MarketsExplorer />;
}
