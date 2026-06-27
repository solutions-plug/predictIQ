import type { ReactNode } from 'react';
import { headers } from 'next/headers';
import { ErrorBoundary } from '../components/ErrorBoundary';
import { darkModeInitScript } from '../lib/darkMode';
import '../styles/accessibility.css';

export const metadata = { title: 'PredictIQ' };

export default async function RootLayout({ children }: { children: ReactNode }) {
  const nonce = (await headers()).get('x-nonce') ?? '';

  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script nonce={nonce} dangerouslySetInnerHTML={{ __html: darkModeInitScript }} />
      </head>
      <body>
        <ErrorBoundary section="main">
          {children}
        </ErrorBoundary>
      </body>
    </html>
  );
}
