-- Off-chain aggregated volumes were stored as DOUBLE PRECISION, which can
-- accumulate floating-point rounding error when many market volumes are
-- summed (e.g. in the /api/v1/statistics and /api/v1/markets/featured
-- aggregate queries). On-chain volumes are already carried as exact
-- NUMERIC-backed strings to avoid this; align the off-chain column so
-- SUM(total_volume) is computed with exact decimal arithmetic instead.
ALTER TABLE markets
    ALTER COLUMN total_volume TYPE NUMERIC USING total_volume::NUMERIC,
    ALTER COLUMN total_volume SET DEFAULT 0;
