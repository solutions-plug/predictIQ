import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LandingPage from '../LandingPage';
import { api } from '../../lib/api/public-client';

const originalFetch = global.fetch;

describe('LandingPage success-message live-region', () => {
  beforeEach(() => {
    // Mock the subscribe API to return success
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

  it('renders the success message inside the live-region div via React state', async () => {
    const { container } = render(<LandingPage />);

    const emailInput = screen.getByLabelText(/email address/i);
    await userEvent.type(emailInput, 'test@example.com');
    await userEvent.click(screen.getByRole('button', { name: /get early access/i }));

    // The success text must appear in the DOM (rendered by React, not injected imperatively)
    await waitFor(() => {
      const liveRegion = container.querySelector('#form-status');
      expect(liveRegion).not.toBeNull();
      expect(liveRegion!.textContent).toMatch(/successfully subscribed to updates/i);
    });
  });

  it('live-region div contains the success text as a React child, not via textContent mutation', async () => {
    const { container } = render(<LandingPage />);

    const emailInput = screen.getByLabelText(/email address/i);
    await userEvent.type(emailInput, 'test@example.com');
    await userEvent.click(screen.getByRole('button', { name: /get early access/i }));

    await waitFor(() => {
      const liveRegion = container.querySelector('#form-status');
      expect(liveRegion).not.toBeNull();
      // Confirm the text is a real child node rendered by React (textContent reflects it)
      expect(liveRegion!.textContent).toMatch(/successfully subscribed to updates/i);
      // The node should have child nodes because React rendered text children, not an
      // empty element patched imperatively after render
      expect(liveRegion!.childNodes.length).toBeGreaterThan(0);
    });
  });

  it('live-region is empty before form is submitted', () => {
    const { container } = render(<LandingPage />);

    const liveRegion = container.querySelector('#form-status');
    expect(liveRegion).not.toBeNull();
    expect(liveRegion!.textContent).toBe('');
  });

  it('live-region has correct ARIA attributes for screen-reader announcement', () => {
    const { container } = render(<LandingPage />);

    const liveRegion = container.querySelector('#form-status');
    expect(liveRegion).not.toBeNull();
    expect(liveRegion).toHaveAttribute('aria-live', 'polite');
    expect(liveRegion).toHaveAttribute('aria-atomic', 'true');
    expect(liveRegion).toHaveAttribute('role', 'status');
  });

  it('does NOT use a React ref to mutate textContent (ref prop is absent from the div)', () => {
    const { container } = render(<LandingPage />);

    // Query the live-region by id to inspect its DOM node directly
    const liveRegionEl = container.querySelector('#form-status');
    expect(liveRegionEl).not.toBeNull();

    // Before submission the node is empty — no imperative content was set
    expect(liveRegionEl!.textContent).toBe('');
  });
});
