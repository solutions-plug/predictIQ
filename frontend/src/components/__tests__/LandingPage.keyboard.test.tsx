import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LandingPage from '../LandingPage';

describe('LandingPage handleKeyDown', () => {
  it('submits the form when Enter is pressed on the form with a valid email', async () => {
    render(<LandingPage />);
    const emailInput = screen.getByLabelText(/email address/i);
    await userEvent.type(emailInput, 'test@example.com');

    const form = emailInput.closest('form')!;
    fireEvent.keyDown(form, { key: 'Enter', code: 'Enter' });

    expect(
      screen.getByRole('button', { name: /already subscribed/i }),
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
      screen.queryByRole('button', { name: /already subscribed/i }),
    ).not.toBeInTheDocument();
  });
});
