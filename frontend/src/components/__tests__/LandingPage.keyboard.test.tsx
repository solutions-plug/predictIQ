import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LandingPage from '../LandingPage';
import { api } from '../../lib/api/client';

const originalFetch = global.fetch;

describe('LandingPage handleKeyDown', () => {
  beforeEach(() => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ success: true, message: 'Subscribed' }),
    });
    jest
      .spyOn(api, 'getStatistics')
      .mockResolvedValue({ totalMarkets: 1, totalVolume: 1, activeUsers: 1 });
  });

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it('submits the form when Enter is pressed on the form with a valid email', async () => {
    render(<LandingPage />);
    const emailInput = screen.getByLabelText(/email address/i);
    await userEvent.type(emailInput, 'test@example.com');

    const form = emailInput.closest('form')!;
    fireEvent.keyDown(form, { key: 'Enter', code: 'Enter' });

    expect(
      await screen.findByRole('button', { name: /subscribed/i }),
    ).toBeInTheDocument();
  });

  it('triggers validation when Enter is pressed on the form with empty email', () => {
    render(<LandingPage />);
    const form = screen.getByLabelText(/email address/i).closest('form')!;

    fireEvent.keyDown(form, { key: 'Enter', code: 'Enter' });

    expect(screen.getByRole('alert')).toHaveTextContent(/email is required/i);
  });

  it('does not trigger submission for non-Enter keys', async () => {
    render(<LandingPage />);
    const emailInput = screen.getByLabelText(/email address/i);
    await userEvent.type(emailInput, 'test@example.com');

    const form = emailInput.closest('form')!;
    fireEvent.keyDown(form, { key: 'a', code: 'KeyA' });

    expect(
      screen.queryByRole('button', { name: /subscribed/i }),
    ).not.toBeInTheDocument();
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
  });
});
