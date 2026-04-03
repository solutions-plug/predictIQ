# PredictIQ Contributor Backlog (40 Issues)

This backlog is based on a direct scan of backend, contracts, and docs code.
Distribution:
- Backend: 30
- Contracts: 7
- Docs: 3

## Backend Issues (30)

1. **Wire admin authentication middleware into admin routes**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/security.rs`
   - Problem: Admin routes are labeled as protected but no API key middleware is attached.
   - Acceptance:
   - `x-api-key` is required for admin endpoints.
   - Invalid or missing key returns `401`.
   - Tests cover authorized and unauthorized calls.

2. **Wire IP whitelist middleware for admin endpoints**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/security.rs`
   - Problem: `IpWhitelist` is initialized but never used.
   - Acceptance:
   - Admin routes enforce whitelist when configured.
   - Non-whitelisted IPs return `403`.
   - Behavior is documented and tested.

3. **Attach security headers middleware globally**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/security.rs`
   - Problem: Security headers middleware exists but is not mounted.
   - Acceptance:
   - Responses include CSP, HSTS, frame, and referrer headers.
   - No duplicate/conflicting headers.
   - Tests validate header presence.

4. **Attach request validation middleware globally**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/validation.rs`
   - Problem: Query/path validation middleware is defined but unused.
   - Acceptance:
   - Suspicious query/path payloads are rejected.
   - Safe traffic is unaffected.
   - Integration tests cover blocked/allowed cases.

5. **Attach content-type validation middleware for mutating routes**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/validation.rs`
   - Problem: Mutation requests are not consistently validated for content type.
   - Acceptance:
   - POST/PUT/PATCH reject unsupported `Content-Type`.
   - JSON endpoints accept `application/json` with charset variants.
   - Tests verify expected status codes.

6. **Attach request size guard middleware**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/validation.rs`
   - Problem: Request body size limits are defined but not enforced.
   - Acceptance:
   - Payloads larger than configured threshold return `413`.
   - Limit is configurable via environment variable.
   - Tests verify boundary behavior.

7. **Fix double newsletter rate limiting policy mismatch**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/rate_limit.rs`, `services/api/src/handlers.rs`
   - Problem: Middleware uses `5/hour` while handler-level limiter uses `5/15min`.
   - Acceptance:
   - Single canonical policy is applied.
   - No duplicate throttling paths.
   - Docs and tests match the final policy.

8. **Move newsletter per-IP limiter to Redis-backed implementation**
   - Area: Backend
   - Files: `services/api/src/newsletter.rs`
   - Problem: In-memory limiter is per-instance and not safe for horizontal scaling.
   - Acceptance:
   - Limits are shared across instances.
   - TTL-based counters are atomic.
   - Load test confirms consistent behavior.

9. **Harden client IP extraction for trusted proxy setups**
   - Area: Backend
   - Files: `services/api/src/security.rs`, `services/api/src/handlers.rs`
   - Problem: `x-forwarded-for` is trusted without proxy trust boundaries.
   - Acceptance:
   - Trusted proxy CIDRs are configurable.
   - Spoofed headers are ignored for untrusted sources.
   - Tests cover direct and proxied deployments.

10. **Restrict CORS from Any/Any/Any to configurable allowlist**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/config.rs`
   - Problem: CORS is currently fully permissive.
   - Acceptance:
   - Allowed origins/methods/headers are configurable.
   - Defaults are secure for production.
   - Preflight and credential behavior is tested.

11. **Protect or gate `/metrics` endpoint**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/handlers.rs`
   - Problem: Prometheus metrics are publicly exposed.
   - Acceptance:
   - Endpoint is protected by auth and/or allowlist.
   - Optional public mode remains configurable.
   - Security behavior is documented.

12. **Implement real market resolve write flow**
   - Area: Backend
   - Files: `services/api/src/handlers.rs`
   - Problem: `resolve_market` endpoint is explicitly a placeholder.
   - Acceptance:
   - Endpoint executes actual resolve workflow or is removed.
   - Cache invalidation follows successful writes only.
   - Error handling and tests are added.

13. **Replace Redis `KEYS` with cursor-based `SCAN` invalidation**
   - Area: Backend
   - Files: `services/api/src/cache/mod.rs`
   - Problem: `del_by_pattern` uses blocking `KEYS` on production-sized datasets.
   - Acceptance:
   - Implementation uses `SCAN` batching.
   - Deletion throughput and safety are validated.
   - Large-keyspace tests added.

14. **Introduce targeted cache invalidation tags**
   - Area: Backend
   - Files: `services/api/src/cache/mod.rs`, `services/api/src/cache/mod.rs` (keys submodule), `services/api/src/handlers.rs`
   - Problem: Invalidations are broad and potentially expensive.
   - Acceptance:
   - Invalidation scope is narrowed to affected entities.
   - Blast radius is measured and reduced.
   - Docs explain key/tag strategy.

15. **Stop swallowing blockchain RPC errors into silent defaults**
   - Area: Backend
   - Files: `services/api/src/blockchain.rs`
   - Problem: RPC failures are often converted to zero/default payloads, masking incidents.
   - Acceptance:
   - Distinguish hard failure vs stale fallback responses.
   - Return observability-friendly status details.
   - Metrics track fallback usage rate.

16. **Introduce explicit contract storage key mapping layer**
   - Area: Backend
   - Files: `services/api/src/blockchain.rs`, `services/api/src/config.rs`
   - Problem: `getContractData` keys are convention-based and may drift from deployed schema.
   - Acceptance:
   - Key schema is centralized and versioned.
   - Per-network key mapping is configurable.
   - Integration tests validate contract reads.

17. **Paginate blockchain events fetch beyond 100-item limit**
   - Area: Backend
   - Files: `services/api/src/blockchain.rs`
   - Problem: Event sync uses fixed limit 100 and can miss events on busy ledgers.
   - Acceptance:
   - Fetch loop paginates until complete range is consumed.
   - Cursor updates are correct under burst load.
   - Regression tests cover >100 events.

18. **Bound watched transaction set growth**
   - Area: Backend
   - Files: `services/api/src/blockchain.rs`
   - Problem: `watched_txs` can grow without TTL/cap for never-finalized hashes.
   - Acceptance:
   - Add TTL and max-size policy for watched hashes.
   - Evictions are observable via metrics/logs.
   - Memory growth test added.

19. **Implement graceful shutdown for background workers**
   - Area: Backend
   - Files: `services/api/src/main.rs`, `services/api/src/blockchain.rs`, `services/api/src/email/queue.rs`
   - Problem: Spawned workers run forever and ignore shutdown signals.
   - Acceptance:
   - Workers stop cleanly on SIGTERM/SIGINT.
   - In-flight work is handled deterministically.
   - Shutdown behavior is tested.

20. **Avoid full in-memory user bet pagination**
   - Area: Backend
   - Files: `services/api/src/blockchain.rs`
   - Problem: Entire `user_bets` list is loaded then paginated, causing high memory and latency.
   - Acceptance:
   - Upstream query supports range/offset or page token.
   - Endpoint avoids full materialization.
   - Performance benchmark shows improvement.

21. **Tighten blockchain health semantics**
   - Area: Backend
   - Files: `services/api/src/blockchain.rs`
   - Problem: `is_healthy` is true when ledger > 0 even if contract read fails.
   - Acceptance:
   - Health reflects both node and contract-read health.
   - Degraded states are represented explicitly.
   - Health endpoint docs updated.

22. **Make DB pool sizing configurable**
   - Area: Backend
   - Files: `services/api/src/db.rs`, `services/api/src/config.rs`
   - Problem: DB pool min/max are hard-coded.
   - Acceptance:
   - Pool sizes and timeouts are env-configurable.
   - Safe defaults remain in place.
   - Config docs updated.

23. **Fix integration test crate target mismatch**
   - Area: Backend
   - Files: `services/api/tests/security_tests.rs`, `services/api/src/main.rs`, `services/api/Cargo.toml`
   - Problem: Tests import `predictiq_api::security` but crate only has a binary target.
   - Acceptance:
   - Add `src/lib.rs` and expose needed modules, or refactor tests.
   - CI runs tests successfully.
   - Project docs include test command.

24. **Verify SendGrid webhook signatures**
   - Area: Backend
   - Files: `services/api/src/email/webhook.rs`, `services/api/src/security.rs`, `services/api/src/config.rs`
   - Problem: Webhook events are accepted without signature validation.
   - Acceptance:
   - Signature/timestamp verification is enforced.
   - Invalid signatures are rejected with `401` or `403`.
   - Replay protection window is implemented.

25. **Separate webhook routing security model from admin routes**
   - Area: Backend
   - Files: `services/api/src/main.rs`
   - Problem: SendGrid webhook is grouped with admin routes; model should be provider-signed, not API-key admin.
   - Acceptance:
   - Webhook route has dedicated middleware stack.
   - Admin auth is not required for valid provider events.
   - Route policy documented in OpenAPI.

26. **Store recipient when logging sent email events**
   - Area: Backend
   - Files: `services/api/src/email/queue.rs`, `services/api/src/db.rs`
   - Problem: `mark_completed` creates `sent` event with empty recipient string.
   - Acceptance:
   - Event records include real recipient address.
   - Existing analytics queries still work.
   - Regression test added.

27. **Recover orphaned processing jobs on startup**
   - Area: Backend
   - Files: `services/api/src/email/queue.rs`
   - Problem: Crash during processing can leave jobs stuck in processing set.
   - Acceptance:
   - Worker startup scans and re-queues stale processing jobs.
   - Idempotent behavior guaranteed.
   - Recovery scenario test added.

28. **Remove memory leak pattern in analytics rate limiter keying**
   - Area: Backend
   - Files: `services/api/src/rate_limit.rs`
   - Problem: `Box::leak` is used for fallback session key path.
   - Acceptance:
   - No leaked allocations for per-request key generation.
   - Behavior remains equivalent.
   - Benchmark shows no regression.

29. **Add structured API error codes and consistent error schema**
   - Area: Backend
   - Files: `services/api/src/handlers.rs`, `services/api/openapi.yaml`
   - Problem: API errors currently expose freeform message only.
   - Acceptance:
   - Error payload includes machine-readable code and message.
   - OpenAPI schemas and responses are updated.
   - All handlers map internal errors consistently.

30. **Sync OpenAPI with runtime configuration and auth**
   - Area: Backend
   - Files: `services/api/openapi.yaml`, `services/api/src/main.rs`
   - Problem: OpenAPI server URL and admin auth model drift from runtime behavior.
   - Acceptance:
   - Server URL defaults align with API bind defaults.
   - Security schemes are declared and applied per route.
   - Contract tests validate spec-vs-runtime route parity.

## Contract Issues (7)

31. **Remove duplicate `cancel_market_admin` entrypoint**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/lib.rs`
   - Problem: Duplicate function definitions exist for the same method.
   - Acceptance:
   - Single canonical function remains.
   - Contract compiles cleanly.
   - ABI remains backward-compatible.

32. **Fix duplicate `get_dispute_window` and dead constant usage**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/modules/resolution.rs`
   - Problem: Two `get_dispute_window` functions exist; one references undefined `DISPUTE_WINDOW_SECONDS`.
   - Acceptance:
   - Single implementation remains.
   - Compile errors are removed.
   - Unit tests cover default and configured values.

33. **Add missing timelock config key and bounds constants**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/types.rs`, `contracts/predict-iq/src/modules/governance.rs`
   - Problem: Governance references `ConfigKey::TimelockDuration`, `TIMELOCK_MIN_SECONDS`, and `TIMELOCK_MAX_SECONDS` but they are not defined.
   - Acceptance:
   - Missing enum variants/constants are added.
   - Timelock setter enforces bounds.
   - Tests validate boundary behavior.

34. **Define missing governance types used by module code**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/types.rs`, `contracts/predict-iq/src/modules/governance.rs`
   - Problem: Governance references `UpgradeStats` and `PendingGuardianRemoval` types that are not present in `types.rs`.
   - Acceptance:
   - Missing structs are defined with contracttype derives.
   - Serialization compatibility is tested.
   - Contract builds and tests pass.

35. **Emit accurate oracle source metadata in oracle result events**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/modules/oracles.rs`, `contracts/predict-iq/src/modules/events.rs`
   - Problem: Oracle event currently reports current contract address instead of real oracle source/ID context.
   - Acceptance:
   - Event includes oracle_id and/or oracle contract source.
   - Indexer-facing schema is documented.
   - Event tests validate payload.

36. **Optimize status query path to avoid full reverse scan**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/modules/queries.rs`, `contracts/predict-iq/src/modules/markets.rs`
   - Problem: `get_markets_by_status` reverse-scans all markets and can become expensive at scale.
   - Acceptance:
   - Add status index keys or equivalent strategy.
   - Query complexity is reduced.
   - Gas benchmark demonstrates improvement.

37. **Add hard bounds for pagination inputs in contract queries**
   - Area: Contracts
   - Files: `contracts/predict-iq/src/modules/queries.rs`
   - Problem: Query functions accept unbounded `limit`, increasing gas/memory risk.
   - Acceptance:
   - Enforce sane max limit constant.
   - Document truncation behavior.
   - Tests validate clamping logic.

## Documentation Issues (3)

38. **Fix broken and stale links in docs index**
   - Area: Docs
   - Files: `docs/README.md`
   - Problem: References include non-existent `docs/security/SECURITY_BEST_PRACTICES.md` and incorrect `docs/ARCHITECTURE.md` path.
   - Acceptance:
   - All links resolve to existing files.
   - Link check script passes.
   - Updated sections reflect current docs tree.

39. **Align database documentation with actual schema and paths**
   - Area: Docs
   - Files: `services/api/DATABASE.md`, `services/api/database/migrations/*.sql`
   - Problem: Docs reference stale absolute path and table names that differ from code usage (`newsletter_subscriptions` vs `newsletter_subscribers`).
   - Acceptance:
   - Table names and migration list match repository state.
   - Commands are workspace-relative and environment-agnostic.
   - Newsletter schema naming is reconciled across docs and code.

40. **Refresh API_SPEC to match current contract and error enums**
   - Area: Docs
   - Files: `API_SPEC.md`, `contracts/predict-iq/src/errors.rs`, `contracts/predict-iq/src/modules/events.rs`
   - Problem: Error codes/event names in API spec are out of sync with on-chain implementation.
   - Acceptance:
   - Error code table reflects current enum values.
   - Event names/topics match emitted events.
   - Method signatures include current multi-oracle parameters.