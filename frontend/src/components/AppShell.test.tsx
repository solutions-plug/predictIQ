import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import AppShell from './AppShell';

vi.mock('next/navigation', () => ({
  usePathname: () => '/',
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), prefetch: vi.fn() }),
}));

vi.mock('next/link', () => ({
  default: ({ children, href, ...rest }: { children: React.ReactNode; href: string }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const ADMIN_KEY = 'predictiq-admin-key';

function mockAdminSession(valid: boolean) {
  return vi.fn().mockResolvedValue({
    ok: valid,
    status: valid ? 200 : 401,
    json: async () => (valid ? { valid: true } : { valid: false }),
  });
}

describe('AppShell admin nav link', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    sessionStorage.clear();
  });

  it('shows the Admin nav item when the stored key is valid', async () => {
    sessionStorage.setItem(ADMIN_KEY, 'valid-key');
    vi.stubGlobal('fetch', mockAdminSession(true));

    render(<AppShell>content</AppShell>);

    expect(await screen.findByRole('link', { name: /admin/i })).toBeInTheDocument();
  });

  it('hides the Admin nav item when the stored key is stale or invalid', async () => {
    sessionStorage.setItem(ADMIN_KEY, 'stale-key');
    vi.stubGlobal('fetch', mockAdminSession(false));

    render(<AppShell>content</AppShell>);

    await waitFor(() => {
      expect(screen.queryByRole('link', { name: /admin/i })).not.toBeInTheDocument();
    });
  });

  it('hides the Admin nav item when no key is stored', async () => {
    const fetchMock = mockAdminSession(false);
    vi.stubGlobal('fetch', fetchMock);

    render(<AppShell>content</AppShell>);

    await waitFor(() => {
      expect(screen.queryByRole('link', { name: /admin/i })).not.toBeInTheDocument();
    });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
