import type { ReactNode } from 'react';
import { headers } from 'next/headers';
import { Orbitron, Exo_2 } from 'next/font/google';
import { ErrorBoundary } from '../components/ErrorBoundary';
import { WalletProvider } from '../lib/wallet/WalletProvider';
import { darkModeInitScript } from '../lib/darkMode';
import '../styles/tokens.css';
import '../styles/accessibility.css';
import '../styles/landing.css';
import '../styles/ui.css';

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

export const metadata = {
  title: 'PredictIQ — Decentralized Prediction Markets on Stellar',
  description:
    'Create, bet on, and resolve prediction markets with transparency, security, and fairness powered by the Stellar blockchain.',
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
        <WalletProvider>
          <ErrorBoundary section="main">{children}</ErrorBoundary>
        </WalletProvider>
      </body>
    </html>
  );
}
