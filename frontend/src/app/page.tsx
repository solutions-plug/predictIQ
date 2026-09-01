import type { Metadata } from 'next';
import dynamic from 'next/dynamic';
import { LoadingSpinner } from '../components/LoadingSpinner';

// Landing-page SEO (#1347). `metadataBase` lets the relative OG/Twitter image
// paths resolve to absolute URLs for link-unfurl crawlers.
const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL ?? 'https://predictiq.app';

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: 'PredictIQ — Decentralized Prediction Markets on Stellar',
  description:
    'Create, bet on, and resolve prediction markets on the Stellar blockchain. Multi-outcome markets, hybrid oracle + community resolution, and instant payouts.',
  alternates: { canonical: '/' },
  openGraph: {
    type: 'website',
    url: SITE_URL,
    siteName: 'PredictIQ',
    title: 'PredictIQ — Decentralized Prediction Markets on Stellar',
    description: 'Create, bet on, and resolve prediction markets on the Stellar blockchain.',
    images: [{ url: '/og-image.png', width: 1200, height: 630, alt: 'PredictIQ' }],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'PredictIQ — Decentralized Prediction Markets on Stellar',
    description: 'Create, bet on, and resolve prediction markets on the Stellar blockchain.',
    images: ['/og-image.png'],
  },
};

// Organization + WebSite JSON-LD. Build-time constant, no user input, no `<`.
const structuredData = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'Organization',
      '@id': SITE_URL + '/#organization',
      name: 'PredictIQ',
      url: SITE_URL,
      logo: SITE_URL + '/icons/logo.svg',
      sameAs: ['https://github.com/solutions-plug/predictIQ'],
    },
    {
      '@type': 'WebSite',
      '@id': SITE_URL + '/#website',
      url: SITE_URL,
      name: 'PredictIQ',
      description: 'Decentralized prediction markets on Stellar.',
      publisher: { '@id': SITE_URL + '/#organization' },
    },
  ],
};

// Code-split, with a single accessible loading fallback. `ssr: true` keeps the
// initial HTML server-rendered for crawlers.
const LandingPage = dynamic(
  () => import('../components/LandingPage').then((mod) => ({ default: mod.LandingPage })),
  { loading: () => <LoadingSpinner aria-label="Loading page" />, ssr: true },
);

export default function Home() {
  return (
    <>
      <script type="application/ld+json">{JSON.stringify(structuredData)}</script>
      <LandingPage />
    </>
  );
}
