import React from 'react';
import './VolumeChart.css';

export interface VolumeChartDatum {
  date: string;
  markets?: number;
  volume?: number;
  active_users?: number;
}

interface ChartPoint {
  date: string;
  value: number;
  x: number;
  y: number;
}

interface SeriesDef {
  key: 'volume' | 'markets' | 'active_users';
  label: string;
  color: string;
  format: (value: number) => string;
}

// One chart per numeric trend the /api/v1/statistics history payload exposes.
// Colors are mode-independent brand tokens (see src/styles/tokens.css).
const SERIES: SeriesDef[] = [
  {
    key: 'volume',
    label: 'Volume',
    color: 'var(--purple)',
    format: (value) => `$${value.toLocaleString()}`,
  },
  {
    key: 'markets',
    label: 'Markets',
    color: 'var(--success)',
    format: (value) => value.toLocaleString(),
  },
  {
    key: 'active_users',
    label: 'Active Users',
    color: 'var(--gold)',
    format: (value) => value.toLocaleString(),
  },
];

const CHART_WIDTH = 600;
const CHART_HEIGHT = 180;
const PAD = { top: 16, right: 16, bottom: 28, left: 52 };
// Cap the per-point markers so a very large dataset stays readable (the line
// itself still uses every point).
const MARKER_CAP = 60;

/**
 * Projects a series' numeric values onto chart coordinates (ordinal x-axis so
 * irregular timestamps can't skew spacing). Rows without a finite value for the
 * series are skipped. A single-point series returns one centered point instead
 * of a degenerate line between identical coordinates.
 */
function buildPoints(data: VolumeChartDatum[], key: SeriesDef['key']): ChartPoint[] {
  const rows: Array<{ date: string; value: number }> = [];
  for (const row of data) {
    const raw = row[key];
    if (typeof raw === 'number' && Number.isFinite(raw)) {
      rows.push({ date: row.date, value: raw });
    }
  }
  if (rows.length === 0) return [];

  const plotWidth = CHART_WIDTH - PAD.left - PAD.right;
  const plotHeight = CHART_HEIGHT - PAD.top - PAD.bottom;

  if (rows.length === 1) {
    return [
      {
        date: rows[0].date,
        value: rows[0].value,
        x: PAD.left + plotWidth / 2,
        y: PAD.top + plotHeight / 2,
      },
    ];
  }

  let min = Infinity;
  let max = -Infinity;

  for (const { value } of rows) {
    if (value < min) min = value;
    if (value > max) max = value;
  }
  if (min === max) {
    // Flat series: pad the range so the line renders mid-chart, not at an edge.
    const pad = Math.abs(max) * 0.1 || 1;
    min -= pad;
    max += pad;
  }
  const range = max - min;

  return rows.map((row, index) => ({
    date: row.date,
    value: row.value,
    x: PAD.left + (index / (rows.length - 1)) * plotWidth,
    y: PAD.top + (1 - (row.value - min) / range) * plotHeight,
  }));
}

function SeriesChart({ def, points }: { def: SeriesDef; points: ChartPoint[] }) {
  const path = points
    .map((point, index) => `${index === 0 ? 'M' : 'L'}${point.x.toFixed(2)},${point.y.toFixed(2)}`)
    .join(' ');

  const markerStep = Math.max(1, Math.ceil(points.length / MARKER_CAP));
  const markers = points.filter((_, index) => index % markerStep === 0 || index === points.length - 1);

  let minValue = Infinity;
  let maxValue = -Infinity;
  for (const { value } of points) {
    if (value < minValue) minValue = value;
    if (value > maxValue) maxValue = value;
  }

  const first = points[0];
  const last = points[points.length - 1];

  return (
    <svg
      className="volume-chart__svg"
      viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
      role="img"
      aria-label={`Line chart of ${def.label.toLowerCase()} over time`}
    >
      <title>{`${def.label} over time`}</title>
      {points.length > 1 && (
        <polyline
          className="volume-chart__line"
          fill="none"
          stroke={def.color}
          strokeWidth={2}
          strokeLinejoin="round"
          strokeLinecap="round"
          points={path}
        />
      )}
      {markers.map((point, index) => (
        <circle key={`${point.date}-${index}`} cx={point.x} cy={point.y} r={3} fill={def.color}>
          <title>{`${point.date}: ${def.format(point.value)}`}</title>
        </circle>
      ))}
      <text className="volume-chart__axis" x={PAD.left} y={PAD.top - 6} textAnchor="middle">
        {def.format(maxValue)}
      </text>
      <text className="volume-chart__axis" x={PAD.left} y={CHART_HEIGHT - PAD.bottom + 12} textAnchor="middle">
        {def.format(minValue)}
      </text>
      <text className="volume-chart__axis" x={PAD.left} y={CHART_HEIGHT - 6} textAnchor="start">
        {first.date}
      </text>
      {last.date !== first.date && (
        <text className="volume-chart__axis" x={CHART_WIDTH - PAD.right} y={CHART_HEIGHT - 6} textAnchor="end">
          {last.date}
        </text>
      )}
    </svg>
  );
}

function SeriesCard({ def, points }: { def: SeriesDef; points: ChartPoint[] }) {
  const [showTable, setShowTable] = React.useState(false);
  const tableId = React.useId();

  return (
    <div className="volume-chart__card">
      <h3 className="volume-chart__card-title">{def.label}</h3>
      {!showTable && <SeriesChart def={def} points={points} />}
      {showTable && (
        <div className="volume-chart__table-wrap" id={tableId}>
          <table className="volume-chart__table">
            <caption>{`${def.label} over time`}</caption>
            <thead>
              <tr>
                <th scope="col">Date</th>
                <th scope="col">{def.label}</th>
              </tr>
            </thead>
            <tbody>
              {points.map((point, index) => (
                <tr key={`${point.date}-${index}`}>
                  <td>{point.date}</td>
                  <td>{def.format(point.value)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <button
        type="button"
        className="volume-chart__toggle"
        aria-expanded={showTable}
        aria-controls={tableId}
        onClick={() => setShowTable((visible) => !visible)}
      >
        {showTable ? 'Hide data table' : 'Show data table'}
      </button>
    </div>
  );
}

interface VolumeChartProps {
  /** Time-series rows, e.g. the `/api/v1/statistics` history payload. */
  data: VolumeChartDatum[];
}

/**
 * Dependency-free SVG time-series charts for volume/market-count/active-user
 * trends, with an accessible data-table toggle as an alternative to each chart
 * (screen readers and non-visual consumers). Renders safely for empty,
 * single-point, and large datasets.
 */
export const VolumeChart: React.FC<VolumeChartProps> = ({ data }) => {
  const cards = SERIES.map((def) => ({ def, points: buildPoints(data, def.key) })).filter(
    (card) => card.points.length > 0,
  );

  if (cards.length === 0) {
    return <p className="volume-chart__empty">No trend data available.</p>;
  }

  return (
    <div className="volume-chart">
      {cards.map(({ def, points }) => (
        <SeriesCard key={def.key} def={def} points={points} />
      ))}
    </div>
  );
};

