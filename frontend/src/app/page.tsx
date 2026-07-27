'use client';
import dynamic from 'next/dynamic';
import { LoadingSpinner } from '../components/LoadingSpinner';

// Dynamic import with code splitting. With ssr: true, next/dynamic manages
// its own loading boundary, so no outer <Suspense> is needed here.
const LandingPage = dynamic(() => import('../components/LandingPage').then(mod => ({ default: mod.LandingPage })), {
  loading: () => <LoadingSpinner aria-label="Loading page" />,
  ssr: true,
});

export default function Home() {
  return <LandingPage />;
}
