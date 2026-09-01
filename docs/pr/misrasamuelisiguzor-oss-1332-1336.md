# API Client: import boundary guard, path-encoding, cache-invalidation, pagination

Four `frontend/src/lib/api/` issues. Three are "regression trap" issues whose fix already
landed in an earlier commit - the delta here is the guard test that keeps it from
regressing, plus one missed call site. #1336 is a new helper.

## What changed and why

### #1332 - public/admin client split guard
The split (`public-client.ts` / `admin-client.ts`, commit `895748e`) and an *export*
boundary test (`public-client.test.ts`) already exist. Added the missing *import* boundary
guard: `client-import-boundary.test.ts` statically scans `src/app/**` and fails if any route
outside `src/app/admin/**` (and the privileged `markets/<id>/resolve` route) imports
`admin-client` directly. It also re-asserts that `public-client.ts` never imports
`admin-client` and exposes no `/api/v1/admin`, `/api/v1/audit`, or `/api/v1/email` path.

### #1333 - centralize path-parameter encoding
`fillPath()` was the intended single encoder (commit `dd027a9`) but lived in
`public-client.ts` and three `src/lib/api/` call sites still called `encodeURIComponent`
directly (`admin-client.ts` email preview, `tts-client.ts` job status + audio).

- Moved `fillPath` (plus a new `fillPathParams` for multi-segment templates) into a
  dedicated `paths.ts`; `public-client.ts` re-exports it so existing importers are
  unaffected.
- Routed the three stray call sites through `fillPath`.
- `path-encoding.test.ts`: (a) a `market_id` / `tx_hash` containing `/`, `?`, `#`
  round-trips as a single encoded path segment (asserted against a mocked `fetch`);
  (b) `fillPath` encodes exactly once; (c) a grep guard - no `src/lib/api/*.ts` file
  except `paths.ts` calls `encodeURIComponent`.
- Out of scope: raw `fetch()` calls in app pages/components that never used the client -
  a broader refactor with its own issues.

### #1335 - invalidate cache tags only on mutation success
The `succeeded` guard (a 200 body with `success: false` must not bust the cache, commit
`4a15eda`) already exists in both request helpers. Added
`cache-invalidation-on-success.test.ts`: a POST returning `{ success: false }` leaves the
tagged entry untouched; `{ success: true }` and non-envelope bodies invalidate as before.
(The guard lives in the request helper, not `cache.ts`, because that is where the response
body is parsed.)

### #1336 - offset/cursor pagination helper (new `pagination.ts`)
- `buildPaginationParams({ mode: 'offset' | 'cursor', limit?, offset?/cursor? })` builds the
  query params for either mode. `limit` defaults to 20 and **throws `RangeError` before the
  request is sent** when it exceeds 100, mirroring the server's documented 400 message.
- The cursor is passed through verbatim - never parsed or mutated client-side.
- `CursorPager` holds the opaque cursor for cursor-mode paging; `setSort(key)` drops the
  stale cursor when the sort order actually changes, so the next page restarts from the top.

## How to test

```
cd frontend
npm ci --legacy-peer-deps
./node_modules/.bin/jest src/lib/api
```

- `src/lib/api` Jest suite: **135 pre-existing + 21 new tests pass**.
- `tsc --noEmit`: the new/changed files add no errors over the repo's pre-existing count.
- `npm run build` (`generate-client && next build`) not run here - needs the full monorepo
  build; nothing in this change touches build config.

## Breaking changes

None. `fillPath` keeps its `public-client` export; the request/cache behaviour is unchanged.

## Related issues

Closes #1332
Closes #1333
Closes #1335
Closes #1336

## PR Checklist

- [x] Branch is up to date with `main`
- [x] Commit messages follow Conventional Commits
- [x] Tests added for the change
- [x] Documentation updated if behaviour changed (n/a - behaviour preserved)
- [x] No secrets or credentials committed
