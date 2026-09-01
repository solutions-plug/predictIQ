import React from 'react';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DeprecationBanner } from '../DeprecationBanner';
import {
  reportResponseHeaders,
  _resetDeprecationForTests,
} from '../../lib/api/deprecation';

function deprecatedHeaders(sunset: string, link?: string): Headers {
  const h = new Headers();
  h.set('Deprecation', 'true');
  h.set('Sunset', sunset);
  if (link) h.set('Link', link);
  return h;
}

describe('DeprecationBanner (#1337)', () => {
  beforeEach(() => {
    _resetDeprecationForTests();
    localStorage.clear();
  });
  afterEach(() => {
    _resetDeprecationForTests();
    localStorage.clear();
  });

  it('renders nothing until the API reports a deprecation', () => {
    render(<DeprecationBanner />);
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('shows the sunset date and migration link once a deprecated response arrives', () => {
    render(<DeprecationBanner />);

    act(() => {
      reportResponseHeaders(
        deprecatedHeaders('2026-07-01', '<https://docs/migrate>; rel="deprecation"'),
      );
    });

    const banner = screen.getByRole('status');
    expect(banner).toHaveTextContent(/support ends/i);
    expect(screen.getByRole('link', { name: /migration guide/i })).toHaveAttribute(
      'href',
      'https://docs/migrate',
    );
  });

  it('dismissal is remembered for that sunset date across remounts', async () => {
    const first = render(<DeprecationBanner />);
    act(() => {
      reportResponseHeaders(deprecatedHeaders('2026-07-01'));
    });
    await userEvent.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    first.unmount();

    // Same sunset date: stays hidden after a fresh mount.
    _resetDeprecationForTests();
    render(<DeprecationBanner />);
    act(() => {
      reportResponseHeaders(deprecatedHeaders('2026-07-01'));
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('a new sunset date brings the banner back after a prior dismissal', () => {
    localStorage.setItem('predictiq.deprecation.dismissed-sunset', '2026-07-01');
    render(<DeprecationBanner />);

    act(() => {
      reportResponseHeaders(deprecatedHeaders('2027-01-01')); // different date
    });

    expect(screen.getByRole('status')).toBeInTheDocument();
  });
});
