'use client';
import dynamic from 'next/dynamic';
import { Suspense } from 'react';
import { LoadingSpinner } from '../components/LoadingSpinner';

// Dynamic import with code splitting
const LandingPage = dynamic(() => import('../components/LandingPage').then(mod => ({ default: mod.LandingPage })), {
  loading: () => <LoadingSpinner aria-label="Loading" />,
  ssr: true,
});

export default function Home() {
  return (
    <Suspense fallback={<LoadingSpinner aria-label="Loading page" />}>
      <LandingPage />
    </Suspense>
  );
}
