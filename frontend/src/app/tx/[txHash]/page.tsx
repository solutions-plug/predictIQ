import { TransactionStatusView } from '@/components/markets/TransactionStatusView';

interface TransactionStatusPageProps {
  params: Promise<{ txHash: string }>;
}

export default async function TransactionStatusPage({ params }: TransactionStatusPageProps) {
  const { txHash } = await params;
  return <TransactionStatusView txHash={decodeURIComponent(txHash)} />;
}
