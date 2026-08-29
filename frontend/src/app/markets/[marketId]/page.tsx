import { MarketDetailView } from '@/components/markets/MarketDetailView';

interface MarketDetailPageProps {
  params: Promise<{ marketId: string }>;
}

export default async function MarketDetailPage({ params }: MarketDetailPageProps) {
  const { marketId } = await params;
  return <MarketDetailView marketId={decodeURIComponent(marketId)} />;
}
