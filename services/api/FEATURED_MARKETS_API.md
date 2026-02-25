# Featured Markets API

## Overview

The Featured Markets API endpoint provides curated, trending prediction markets for display on the landing page and throughout the application. Markets are ranked using a sophisticated algorithm that considers trading volume, participant count, time remaining, and category diversity.

## Endpoint

```
GET /api/v1/markets/featured
```

## Query Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `category` | string | No | - | Filter markets by category (e.g., "crypto", "politics", "sports") |
| `limit` | integer | No | 8 | Number of markets to return (1-20) |
| `page` | integer | No | 1 | Page number for pagination (1-based) |

## Response Format

```typescript
{
  markets: Market[],
  total: number,
  page: number,
  page_size: number,
  last_updated: string  // ISO 8601 timestamp
}
```

### Market Object

```typescript
interface Market {
  id: number;
  title: string;
  description: string | null;
  category: string;
  volume: number;
  participant_count: number;
  ends_at: string;  // ISO 8601 timestamp
  outcome_options: any[];  // Array of possible outcomes
  current_odds: Record<string, number>;  // Odds for each outcome
  onchain_volume: string;  // Blockchain-verified volume
  resolved_outcome: number | null;  // Winning outcome if resolved
}
```

## Examples

### Get Top 8 Featured Markets

```bash
curl "http://localhost:8080/api/v1/markets/featured"
```

Response:
```json
{
  "markets": [
    {
      "id": 3,
      "title": "US Presidential Election 2026 Midterms",
      "description": "Which party will control the US House of Representatives after 2026 midterms?",
      "category": "politics",
      "volume": 210000.0,
      "participant_count": 890,
      "ends_at": "2026-11-03T23:59:59Z",
      "outcome_options": ["Democrats", "Republicans", "Other"],
      "current_odds": {
        "Democrats": 0.48,
        "Republicans": 0.50,
        "Other": 0.02
      },
      "onchain_volume": "210000000000",
      "resolved_outcome": null
    }
  ],
  "total": 12,
  "page": 1,
  "page_size": 8,
  "last_updated": "2026-02-25T10:30:00Z"
}
```

### Filter by Category

```bash
curl "http://localhost:8080/api/v1/markets/featured?category=crypto&limit=6"
```

### Pagination

```bash
curl "http://localhost:8080/api/v1/markets/featured?page=2&limit=6"
```

## Ranking Algorithm

Markets are ranked using the following criteria (in order of priority):

1. **Trading Volume** (Primary): Markets with higher total volume are ranked first
2. **Participant Count** (Secondary): More participants indicate higher engagement
3. **Time Remaining** (Tertiary): Markets ending sooner are prioritized for urgency

The SQL query implements this as:
```sql
ORDER BY total_volume DESC, participant_count DESC, ends_at ASC
```

## Caching

- Cache TTL: 2 minutes
- Cache keys include category, page, and limit parameters
- Automatic cache invalidation on market updates
- Separate caching layers for database and blockchain data

## Performance

- Database queries use optimized indexes on:
  - `status` (for active markets filter)
  - `category` (for category filtering)
  - `total_volume DESC` (for ranking)
  - `participant_count DESC` (for ranking)
  - Composite index on `(status, total_volume, participant_count, ends_at)`

- Response times:
  - Cache hit: < 10ms
  - Cache miss: 50-150ms (includes blockchain data fetch)

## Categories

Available market categories:
- `crypto` - Cryptocurrency markets
- `politics` - Political events and elections
- `technology` - Tech industry predictions
- `sports` - Sports events and outcomes
- `stocks` - Stock market predictions
- `space` - Space exploration and missions
- `climate` - Climate and environmental predictions
- `entertainment` - Entertainment industry predictions

## Error Responses

### 400 Bad Request
```json
{
  "message": "Invalid query parameters"
}
```

### 500 Internal Server Error
```json
{
  "message": "Database connection failed"
}
```

## Rate Limiting

No specific rate limiting is applied to this endpoint, but general API rate limits apply:
- 100 requests per minute per IP
- Cached responses don't count toward blockchain RPC limits

## Integration Example

### TypeScript/React

```typescript
interface FeaturedMarketsResponse {
  markets: Market[];
  total: number;
  page: number;
  page_size: number;
  last_updated: string;
}

async function fetchFeaturedMarkets(
  category?: string,
  limit: number = 8,
  page: number = 1
): Promise<FeaturedMarketsResponse> {
  const params = new URLSearchParams();
  if (category) params.append('category', category);
  params.append('limit', limit.toString());
  params.append('page', page.toString());

  const response = await fetch(
    `/api/v1/markets/featured?${params.toString()}`
  );
  
  if (!response.ok) {
    throw new Error('Failed to fetch featured markets');
  }
  
  return response.json();
}

// Usage
const { markets, total } = await fetchFeaturedMarkets('crypto', 6);
```

## Database Schema

The endpoint queries the `markets` table:

```sql
CREATE TABLE markets (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    total_volume DOUBLE PRECISION NOT NULL DEFAULT 0,
    participant_count INTEGER NOT NULL DEFAULT 0,
    ends_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    outcome_options JSONB NOT NULL DEFAULT '[]'::jsonb,
    current_odds JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);
```

## Testing

Run the API tests:
```bash
cd services/api
cargo test featured_markets
```

Load seed data:
```bash
psql $DATABASE_URL -f database/seeds/001_seed_markets.sql
```

## Monitoring

Metrics are exposed at `/metrics` endpoint:
- `api_featured_markets_requests_total` - Total requests
- `api_featured_markets_cache_hits_total` - Cache hit count
- `api_featured_markets_cache_misses_total` - Cache miss count
- `api_featured_markets_duration_seconds` - Request duration histogram

## Future Enhancements

- [ ] Add personalized ranking based on user preferences
- [ ] Implement A/B testing for ranking algorithms
- [ ] Add trending score based on recent activity
- [ ] Support for multiple sorting options (volume, participants, ending soon)
- [ ] Real-time updates via WebSocket
- [ ] Machine learning-based recommendations
