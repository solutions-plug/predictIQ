import React from 'react';
import { render, screen, act } from '@testing-library/react';

let mockPath = '/markets';
jest.mock('next/navigation', () => ({ usePathname: () => mockPath }));
jest.mock('next/link', () => ({
  __esModule: true,
  default: ({ children, href, ...rest }: any) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));
// AppShell imports this hook but it does not exist on main; provide a virtual stand-in.
jest.mock(
  '../../hooks/useActiveNavItem',
  () => ({
    useActiveNavItem: (p: string) => (href: string) => p === href,
  }),
  { virtual: true },
);
jest.mock('../../styles/ui.css', () => ({}), { virtual: true });
jest.mock('../OfflineBanner.css', () => ({}), { virtual: true });

import AppShell from '../AppShell';
import { OfflineBanner } from '../OfflineBanner';

const setOnline = (v: boolean) =>
  Object.defineProperty(window.navigator, 'onLine', { value: v, configurable: true });

describe('AppShell layout', () => {
  beforeEach(() => {
    sessionStorage.clear();
    jest.useFakeTimers();
  });
  afterEach(() => {
    jest.useRealTimers();
    setOnline(true);
  });

  it('marks the nav item for the current route active and updates on route change', () => {
    mockPath = '/markets';
    const { rerender } = render(<AppShell>x</AppShell>);
    expect(screen.getByRole('link', { name: 'Markets' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Statistics' })).not.toHaveAttribute('aria-current');

    mockPath = '/statistics';
    rerender(<AppShell>x</AppShell>);
    expect(screen.getByRole('link', { name: 'Statistics' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Markets' })).not.toHaveAttribute('aria-current');
  });

  it('shows the offline banner alongside the shell when offline', () => {
    setOnline(false);
    render(
      <>
        <OfflineBanner />
        <AppShell>x</AppShell>
      </>,
    );
    act(() => {
      jest.advanceTimersByTime(600);
    });
    expect(screen.getByRole('status')).toHaveTextContent(/offline/i);
  });

  it('has no banner when online', () => {
    setOnline(true);
    render(
      <>
        <OfflineBanner />
        <AppShell>x</AppShell>
      </>,
    );
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });
});
