import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { VolumeChart, type VolumeChartDatum } from '../statistics/VolumeChart';

describe('VolumeChart', () => {
  it('renders an empty state when there is no trend data', () => {
    render(<VolumeChart data={[]} />);

    expect(screen.getByText('No trend data available.')).toBeInTheDocument();
  });

  it('renders one chart card per series that has data', () => {
    const data: VolumeChartDatum[] = [
      { date: '2026-08-01', volume: 1200, markets: 8 },
      { date: '2026-08-02', volume: 1400, markets: 9 },
    ];
    render(<VolumeChart data={data} />);

    expect(screen.getByRole('heading', { name: 'Volume' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Markets' })).toBeInTheDocument();
    // No active_users in the payload -> no card for that series.
    expect(screen.queryByRole('heading', { name: 'Active Users' })).not.toBeInTheDocument();

    expect(screen.getByRole('img', { name: /line chart of volume over time/i })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /line chart of markets over time/i })).toBeInTheDocument();
  });

  it('skips rows that lack the value for a given series', () => {
    const data: VolumeChartDatum[] = [
      { date: '2026-08-01', volume: 1200 },
      { date: '2026-08-02', volume: 1400, markets: 9 },
      { date: '2026-08-03', volume: 1500, markets: 10 },
    ];
    render(<VolumeChart data={data} />);

    expect(screen.getByRole('img', { name: /line chart of volume over time/i })).toBeInTheDocument();
    // markets is present on only 2 rows (still >= 1 point) so its chart renders.
    expect(screen.getByRole('img', { name: /line chart of markets over time/i })).toBeInTheDocument();
  });

  it('handles a single-point dataset without a degenerate line', () => {
    const data: VolumeChartDatum[] = [{ date: '2026-08-01', volume: 500, markets: 3 }];
    render(<VolumeChart data={data} />);

    const chart = screen.getByRole('img', { name: /line chart of volume over time/i });
    expect(chart).toBeInTheDocument();
    // Single point: no polyline (M/L path) is drawn.
    expect(chart.querySelector('polyline')).not.toBeInTheDocument();
    expect(chart.querySelectorAll('circle')).toHaveLength(1);
  });

  it('toggles the accessible data table for a chart', () => {
    const data: VolumeChartDatum[] = [
      { date: '2026-08-01', volume: 1200, markets: 8 },
      { date: '2026-08-02', volume: 1400, markets: 9 },
    ];
    render(<VolumeChart data={data} />);

    const toggle = screen.getAllByRole('button', { name: 'Show data table' })[0];
    fireEvent.click(toggle);

    const table = screen.getByRole('table', { name: 'Volume over time' });
    expect(table).toBeInTheDocument();
    expect(screen.getByText('$1,200')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Hide data table' }));
    expect(screen.queryByRole('table', { name: 'Volume over time' })).not.toBeInTheDocument();
  });
});
