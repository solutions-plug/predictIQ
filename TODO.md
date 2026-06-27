# TODO

## Plan to implement validated Correlation ID handling

1. Inspect current correlation middleware implementation in `services/api/src/correlation.rs`.
2. Add validation logic:
   - enforce maximum header value length
   - accept only UUID v4 (parse + version check)
3. If missing/invalid/too long: generate new UUID v4.
4. Ensure the normalized UUID is recorded in tracing span and echoed back via `X-Request-Id` response header.
5. Add/adjust unit tests for the middleware to cover:
   - valid UUID v4 passes through
   - malformed string replaced
   - UUID v1/other versions replaced
   - too-long header replaced
6. Update any documentation/comments if needed.
7. Run `cargo test -p services/api` (or workspace-equivalent) to confirm tests pass.

