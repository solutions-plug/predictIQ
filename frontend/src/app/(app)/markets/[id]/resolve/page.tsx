'use client';

import { useParams } from 'next/navigation';
import { ResolvePanel } from '../../../../../components/market-resolve/ResolvePanel';

export default function ResolveMarketPage() {
  const params = useParams<{ id: string }>();
  const id = Array.isArray(params?.id) ? params.id[0] : params?.id;

  if (!id) {
    return null;
  }

  return <ResolvePanel id={id} />;
}
