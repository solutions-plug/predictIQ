import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AdminLayout from './layout';

describe('AdminLayout API key rotation', () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it('does not send a rotate request when the operator cancels', async () => {
    const fetchMock = jest.fn();
    global.fetch = fetchMock as unknown as typeof fetch;

    render(<AdminLayout>{null}</AdminLayout>);

    await userEvent.click(screen.getByRole('button', { name: /rotate/i }));

    expect(screen.getByRole('dialog')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /cancel/i }));

    expect(fetchMock).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('sends the rotate request with a body and shows the new key on confirm', async () => {
    const newKey = 'sk_live_new_key_123';
    const fetchMock = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ key: newKey }),
    });
    global.fetch = fetchMock as unknown as typeof fetch;

    render(<AdminLayout>{null}</AdminLayout>);

    await userEvent.click(screen.getByRole('button', { name: /rotate/i }));
    await userEvent.click(screen.getByRole('button', { name: /confirm/i }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toMatch(/rotate/i);
    expect(init).toBeDefined();
    expect(init.method).toBe('POST');
    expect(init.body).toBeDefined();
    expect(JSON.parse(init.body)).toEqual({ confirm: true });

    expect(await screen.findByText(newKey)).toBeInTheDocument();
  });
});
