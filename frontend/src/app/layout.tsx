import type { ReactNode } from 'react';
import { headers } from 'next/headers';
import { Orbitron, Exo_2 } from 'next/font/google';
import { ErrorBoundary } from '../components/ErrorBoundary';
import { OfflineBanner } from '../components/OfflineBanner';
import { AxeAccessibility } from '../components/AxeAccessibility';
import { WalletProvider } from '../lib/wallet/WalletProvider';
import { darkModeInitScript } from '../lib/darkMode';
import '../styles/tokens.css';
import '../styles/accessibility.css';
import '../styles/landing.css';

// Self-hosted at build time so the strict CSP (font-src 'self') is satisfied
// without whitelisting the Google Fonts CDN.
const display = Orbitron({
  subsets: ['latin'],
  weight: ['500', '600', '700'],
  variable: '--font-display',
  display: 'swap',
});

const body = Exo_2({
  subsets: ['latin'],
  weight: ['300', '400', '500', '600', '700'],
  variable: '--font-body',
  display: 'swap',
});

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL ?? 'https://predictiq.app';

// Site-wide metadata defaults. Per-route pages (e.g. app/page.tsx) override
// title/description and add route-specific Open Graph data.
export const metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: 'PredictIQ — Decentralized Prediction Markets on Stellar',
    template: '%s · PredictIQ',
  },
  description:
    'Create, bet on, and resolve prediction markets with transparency, security, and fairness powered by the Stellar blockchain.',
  applicationName: 'PredictIQ',
  openGraph: {
    type: 'website',
    siteName: 'PredictIQ',
    url: SITE_URL,
  },
  twitter: {
    card: 'summary_large_image',
  },
};

export default async function RootLayout({ children }: { children: ReactNode }) {
  const nonce = (await headers()).get('x-nonce') ?? '';

  return (
    <html
      lang="en"
      className={`${display.variable} ${body.variable}`}
      suppressHydrationWarning
    >
      <head>
        <script nonce={nonce} dangerouslySetInnerHTML={{ __html: darkModeInitScript }} />
      </head>
      <body>
        {/* Dev-only @axe-core/react checker; tree-shaken out of prod bundles. */}
        <AxeAccessibility />
        <OfflineBanner />
        <ErrorBoundary section="main">
          <WalletProvider>
            <AppShell>{children}</AppShell>
          </WalletProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}
