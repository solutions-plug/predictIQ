import {
  buildPaginationParams,
  CursorPager,
  DEFAULT_LIMIT,
  MAX_LIMIT,
} from '../pagination';

describe('buildPaginationParams', () => {
  it('defaults limit to 20 in offset mode', () => {
    expect(buildPaginationParams({ mode: 'offset' })).toEqual({ limit: String(DEFAULT_LIMIT) });
  });

  it('emits offset only when non-zero', () => {
    expect(buildPaginationParams({ mode: 'offset', offset: 0 })).toEqual({ limit: '20' });
    expect(buildPaginationParams({ mode: 'offset', limit: 50, offset: 100 })).toEqual({
      limit: '50',
      offset: '100',
    });
  });

  it('passes an opaque cursor through unchanged', () => {
    const cursor = 'eyJpZCI6NDJ9';
    expect(buildPaginationParams({ mode: 'cursor', cursor })).toEqual({ limit: '20', cursor });
  });

  it('omits the cursor param on the first cursor-mode page', () => {
    expect(buildPaginationParams({ mode: 'cursor' })).toEqual({ limit: '20' });
  });

  it('throws before sending when limit exceeds the server maximum', () => {
    expect(() => buildPaginationParams({ mode: 'offset', limit: MAX_LIMIT + 1 })).toThrow(RangeError);
    expect(() => buildPaginationParams({ mode: 'cursor', limit: 500 })).toThrow(
      /exceeds the maximum allowed value of 100/,
    );
  });

  it('rejects a non-positive limit and a negative offset', () => {
    expect(() => buildPaginationParams({ mode: 'offset', limit: 0 })).toThrow(RangeError);
    expect(() => buildPaginationParams({ mode: 'offset', limit: 1.5 })).toThrow(RangeError);
    expect(() => buildPaginationParams({ mode: 'offset', offset: -1 })).toThrow(RangeError);
  });
});

describe('CursorPager', () => {
  it('advances through cursors and reports position', () => {
    const pager = new CursorPager('created_at');
    expect(pager.atStart).toBe(true);
    expect(pager.params()).toEqual({ limit: '20' });

    pager.advance('cursor-page-2');
    expect(pager.atStart).toBe(false);
    expect(pager.params(50)).toEqual({ limit: '50', cursor: 'cursor-page-2' });

    pager.advance(null); // end of list
    expect(pager.atStart).toBe(true);
  });

  it('discards the stale cursor when the sort order changes', () => {
    const pager = new CursorPager('created_at');
    pager.advance('cursor-mid-list');
    expect(pager.atStart).toBe(false);

    pager.setSort('volume'); // opaque cursor is no longer valid for the new order
    expect(pager.atStart).toBe(true);
    expect(pager.params()).toEqual({ limit: '20' });
  });

  it('keeps the cursor when setSort is called with the same key', () => {
    const pager = new CursorPager('created_at');
    pager.advance('cursor-mid-list');
    pager.setSort('created_at');
    expect(pager.atStart).toBe(false);
  });
});
