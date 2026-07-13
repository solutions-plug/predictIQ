'use client';

import { useParams } from 'next/navigation';
import { MarketDetail } from '../../../../components/market-detail/MarketDetail';

/** Route entry for a single market. Reads the [id] param and delegates to the client detail view. */
export default function MarketDetailPage() {
  const params = useParams<{ id: string }>();
  const id = Array.isArray(params.id) ? params.id[0] : params.id;
  return <MarketDetail id={id} />;
}
