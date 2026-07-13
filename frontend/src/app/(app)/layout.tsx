import type { ReactNode } from 'react';
import { AppHeader } from '../../components/app/AppHeader';

/** Layout shared by every authenticated app surface (markets, portfolio, …). */
export default function AppLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <AppHeader />
      <main id="main-content" className="app-container" style={{ paddingBottom: '5rem' }}>
        {children}
      </main>
    </>
  );
}
