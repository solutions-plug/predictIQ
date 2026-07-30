import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LandingPage from '../LandingPage';
import { api } from '../../lib/api/public-client';

const originalFetch = global.fetch;

describe('LandingPage Enter-to-submit', () => {
  beforeEach(() => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ success: true, message: 'Subscribed' }),
    });
    jest
      .spyOn(api, 'getStatistics')
      .mockResolvedValue({ total_markets: 1, total_volume: 1, active_markets: 1 });
  });

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  // Regression test for a double-submit bug: an explicit onKeyDown handler
  // called requestSubmit() on Enter without calling preventDefault(), so in
  // a real browser the native implicit-submit-on-Enter behavior for a
  // single-input form fired *as well*, submitting twice. fireEvent.keyDown
  // does not exercise that native browser behavior (jsdom only triggers
  // implicit submission from a real keypress sequence), so this uses
  // userEvent to type into the field and press Enter as a genuine keypress.
  it('submits the newsletter form exactly once when Enter is pressed in the email field', async () => {
    render(<LandingPage />);
    const emailInput = screen.getByLabelText(/email address/i);

    await userEvent.type(emailInput, 'test@example.com{Enter}');

    await screen.findByRole('button', { name: /subscribed/i });

    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it('triggers validation when Enter is pressed with an empty email, without calling the API', async () => {
    render(<LandingPage />);
    const emailInput = screen.getByLabelText(/email address/i);
    emailInput.focus();

    await userEvent.keyboard('{Enter}');

    expect(screen.getByRole('alert')).toHaveTextContent(/email is required/i);
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('keeps keyboard focus on the submit button through loading and success, instead of silently dropping it', async () => {
    render(<LandingPage />);
    const emailInput = screen.getByLabelText(/email address/i);
    const submitButton = screen.getByRole('button', { name: /get early access/i });

    await userEvent.type(emailInput, 'test@example.com');
    await userEvent.tab();
    expect(submitButton).toHaveFocus();

    await userEvent.keyboard('{Enter}');

    // While the request is in-flight the button is aria-disabled (not natively
    // disabled), so it stays in the document and keeps focus.
    expect(submitButton).toHaveAttribute('aria-disabled', 'true');
    expect(submitButton).toHaveFocus();

    await screen.findByRole('button', { name: /subscribed/i });
    expect(submitButton).toHaveFocus();
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });
});
