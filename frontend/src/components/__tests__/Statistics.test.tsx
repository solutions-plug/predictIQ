import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { Statistics } from '../Statistics';
import { api } from '../../lib/api/client';

// Mock the API
jest.mock('../../lib/api/client', () => ({
  api: {
    getStatistics: jest.fn(),
  },
}));

const mockApi = api as jest.Mocked<typeof api>;

describe('Statistics', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('shows loading state initially', () => {
    mockApi.getStatistics.mockResolvedValue({});
    render(<Statistics />);

    expect(screen.getByRole('status', { name: /loading statistics data/i })).toBeInTheDocument();
    expect(screen.getAllByRole('status', { name: /loading/i })).toHaveLength(4); // 3 skeletons + 1 spinner
  });

  it('displays data when loaded successfully', async () => {
    const mockData = {
      totalMarkets: 150,
      totalVolume: 2500000,
      activeUsers: 50000,
    };
    mockApi.getStatistics.mockResolvedValue(mockData);

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.getByText('150')).toBeInTheDocument();
    });

    expect(screen.getByText('$2,500,000')).toBeInTheDocument();
    expect(screen.getByText('50,000')).toBeInTheDocument();
    expect(screen.queryByRole('status', { name: /loading/i })).not.toBeInTheDocument();
  });

  it('shows error state and retry button on failure', async () => {
    const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
    mockApi.getStatistics.mockRejectedValue(new Error('Network error'));

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.getByText(/failed to load statistics/i)).toBeInTheDocument();
    });

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();

    consoleSpy.mockRestore();
  });

  it('retries on button click', async () => {
    mockApi.getStatistics
      .mockRejectedValueOnce(new Error('Network error'))
      .mockResolvedValueOnce({ totalMarkets: 100 });

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /retry/i }));

    await waitFor(() => {
      expect(screen.getByText('100')).toBeInTheDocument();
    });

    expect(mockApi.getStatistics).toHaveBeenCalledTimes(2);
  });

  it('has proper accessibility attributes', () => {
    mockApi.getStatistics.mockResolvedValue({});
    render(<Statistics />);

    expect(screen.getByRole('region', { name: /platform statistics/i })).toBeInTheDocument();
  });
});