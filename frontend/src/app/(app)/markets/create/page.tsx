import type { Metadata } from 'next';
import { CreateMarketForm } from '../../../../components/market-create/CreateMarketForm';

export const metadata: Metadata = {
  title: 'Create market · PredictIQ',
  description: 'Launch a new prediction market on PredictIQ.',
};

export default function CreateMarketPage() {
  return <CreateMarketForm />;
}
