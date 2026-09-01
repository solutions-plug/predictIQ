/**
 * Shared pagination helper for list endpoints.
 *
 * `API_SPEC.md` documents two modes with one shared `limit` (default 20, max 100;
 * `limit > 100` is a 400 on the server). Every list-consuming page builds its query
 * params through here so the cap is enforced client-side in exactly one place, and so
 * the offset/cursor param shapes never drift page to page.
 */

export const DEFAULT_LIMIT = 20;
export const MAX_LIMIT = 100;

// === Types

export interface OffsetPageRequest {
  mode: 'offset';
  limit?: number;
  offset?: number;
}

export interface CursorPageRequest {
  mode: 'cursor';
  limit?: number;
  /** Opaque server-issued cursor. Never inspected or mutated client-side. */
  cursor?: string;
}

export type PageRequest = OffsetPageRequest | CursorPageRequest;

// === Builder

function validateLimit(limit: number): void {
  if (!Number.isInteger(limit) || limit < 1) {
    throw new RangeError(`limit must be a positive integer, got ${limit}.`);
  }
  if (limit > MAX_LIMIT) {
    // Mirror the server's documented 400 message so the client-side failure reads
    // the same as the one it is pre-empting.
    throw new RangeError(`limit ${limit} exceeds the maximum allowed value of ${MAX_LIMIT}.`);
  }
}

/**
 * Build the query params for a paginated request, clamping/validating `limit` before
 * anything is sent. Throws `RangeError` for an out-of-range `limit` or a negative
 * `offset` - the request never leaves the client.
 */
export function buildPaginationParams(req: PageRequest): Record<string, string> {
  const limit = req.limit ?? DEFAULT_LIMIT;
  validateLimit(limit);

  const params: Record<string, string> = { limit: String(limit) };

  if (req.mode === 'offset') {
    const offset = req.offset ?? 0;
    if (!Number.isInteger(offset) || offset < 0) {
      throw new RangeError(`offset must be a non-negative integer, got ${offset}.`);
    }
    if (offset > 0) params.offset = String(offset);
  } else if (req.cursor) {
    // Passed through verbatim - the cursor's contents are the server's concern.
    params.cursor = req.cursor;
  }

  return params;
}

// === Cursor pager

/**
 * Stateful helper for cursor-mode pagination. An opaque cursor is only valid for the
 * sort order it was issued under, so changing the sort discards it and the next page
 * request restarts from the beginning.
 */
export class CursorPager {
  private cursor: string | null = null;
  private sortKey: string;

  constructor(sortKey = '') {
    this.sortKey = sortKey;
  }

  /** Query params for the next page in the current position. */
  params(limit?: number): Record<string, string> {
    return buildPaginationParams({
      mode: 'cursor',
      limit,
      cursor: this.cursor ?? undefined,
    });
  }

  /** Record the `next_cursor` returned by the last page (or `null` at the end). */
  advance(nextCursor: string | null | undefined): void {
    this.cursor = nextCursor ?? null;
  }

  /** Change the sort order. If it actually changed, the stale cursor is dropped. */
  setSort(sortKey: string): void {
    if (sortKey !== this.sortKey) {
      this.sortKey = sortKey;
      this.cursor = null;
    }
  }

  /** True when positioned at the first page (no cursor held). */
  get atStart(): boolean {
    return this.cursor === null;
  }
}
