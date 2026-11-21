import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import StatisticsPage from '../page';
import { api } from '../../../lib/api/public-client';

jest.mock('next/navigation', () => ({
  useRouter: () => ({ push: jest.fn(), replace: jest.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));
jest.mock('../../../lib/api/public-client', () => ({ api: { getStatistics: jest.fn() } }));
jest.mock('../../../components/statistics/VolumeChart', () => ({ VolumeChart: () => <div /> }));
jest.mock('../../../components/statistics/ExportButton', () => ({ ExportButton: () => null }));

const getStatistics = api.getStatistics as jest.Mock;

describe('StatisticsPage empty and error states (#1931)', () => {
  beforeEach(() => getStatistics.mockReset());

  it('renders the empty state for an empty response and leaves loading', async () => {
    getStatistics.mockResolvedValue({});
    render(<StatisticsPage />);

    expect(await screen.findByText('No category data for the current filter.')).toBeInTheDocument();
    expect(screen.getByText('No history data for the selected date range.')).toBeInTheDocument();
    expect(screen.queryByLabelText('Loading statistics')).not.toBeInTheDocument();
  });

  it('renders the error state when the fetch fails rather than hanging in loading', async () => {
    getStatistics.mockRejectedValue(new Error('boom'));
    render(<StatisticsPage />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to load statistics');
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByLabelText('Loading statistics')).not.toBeInTheDocument());
  });
});
