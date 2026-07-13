import { MARKET_CATEGORIES, type MarketStatus } from '../../lib/types';
import { Field, Input, Select } from '../ui';

export type StatusFilter = MarketStatus | '';

export interface MarketFilterValues {
  category: string;
  status: StatusFilter;
  query: string;
}

interface MarketFiltersProps {
  values: MarketFilterValues;
  onCategoryChange: (category: string) => void;
  onStatusChange: (status: StatusFilter) => void;
  onQueryChange: (query: string) => void;
}

export function MarketFilters({
  values,
  onCategoryChange,
  onStatusChange,
  onQueryChange,
}: MarketFiltersProps) {
  return (
    <form className="market-filters" role="search" onSubmit={(e) => e.preventDefault()}>
      <div className="market-filters__search">
        <Field label="Search markets" htmlFor="market-search">
          <Input
            id="market-search"
            type="search"
            inputMode="search"
            placeholder="Search by title or description…"
            value={values.query}
            onChange={(e) => onQueryChange(e.target.value)}
          />
        </Field>
      </div>

      <Field label="Category" htmlFor="market-category">
        <Select
          id="market-category"
          value={values.category}
          onChange={(e) => onCategoryChange(e.target.value)}
        >
          <option value="">All categories</option>
          {MARKET_CATEGORIES.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </Select>
      </Field>

      <Field label="Status" htmlFor="market-status">
        <Select
          id="market-status"
          value={values.status}
          onChange={(e) => onStatusChange(e.target.value as StatusFilter)}
        >
          <option value="">All statuses</option>
          <option value="open">Open</option>
          <option value="resolved">Resolved</option>
        </Select>
      </Field>
    </form>
  );
}
