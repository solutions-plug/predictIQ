# Graceful Shutdown Implementation

This document describes the graceful shutdown implementation for the PredictIQ API backend workers.

## Overview

The application now supports graceful shutdown of all background workers when receiving termination signals (SIGTERM, SIGINT, Ctrl+C). This ensures that:

- In-flight work is completed or properly handled
- Workers stop cleanly without data loss
- Shutdown completes within a reasonable timeout (30 seconds by default)
- Application state is properly saved before termination

## Architecture

### Components

1. **ShutdownCoordinator** (`src/shutdown.rs`)
   - Coordinates shutdown across all workers
   - Uses broadcast channels to signal shutdown
   - Tracks worker completion with watch channels
   - Implements timeout handling

2. **WorkerHandle** (`src/shutdown.rs`)
   - Wraps individual worker tasks
   - Provides join/abort functionality
   - Reports completion to coordinator

3. **Signal Handlers** (`src/shutdown.rs`)
   - Cross-platform signal handling (Unix/Windows)
   - Listens for SIGTERM, SIGINT, Ctrl+C, etc.

### Workers

The following background workers support graceful shutdown:

1. **Blockchain Sync Worker** (`src/blockchain.rs`)
   - Saves sync cursor before shutdown
   - Completes current sync operation
   - Logs remaining state

2. **Blockchain Transaction Monitor** (`src/blockchain.rs`)
   - Logs watched transactions count
   - Stops monitoring gracefully

3. **Email Queue Worker** (`src/email/queue.rs`)
   - Processes remaining retries
   - Reports jobs still in processing state
   - Completes current job processing

4. **Rate Limiter Cleanup** (`src/lib.rs`)
   - Performs final cleanup
   - Stops periodic maintenance

## Usage

### Normal Operation

The application automatically handles graceful shutdown when receiving termination signals:

```bash
# Send SIGTERM (recommended)
kill -TERM <pid>

# Send SIGINT (Ctrl+C)
kill -INT <pid>
```

### Configuration

Environment variables that affect shutdown behavior:

- `RUST_LOG`: Set to `info` or `debug` to see shutdown logs
- No specific shutdown timeout configuration (hardcoded to 30 seconds)

### Monitoring

The enhanced health endpoint (`/health`) provides worker status:

```json
{
  "status": "ok",
  "timestamp": "2024-03-30T10:00:00Z",
  "workers": {
    "blockchain_sync": "running",
    "blockchain_monitor": "running",
    "email_queue": "running",
    "rate_limiter_cleanup": "running",
    "email_queue_processing": 0
  }
}
```

## Testing

### Unit Tests

Run the shutdown-specific tests:

```bash
cargo test shutdown_tests
```

The tests in `src/shutdown_tests.rs` that cover the shutdown sequence include:

- `test_shutdown_timeout_handling` — verifies that the coordinator gives up
  waiting for workers once the timeout budget expires and logs the error.
- `test_multiple_workers_coordination` — verifies that multiple workers are
  signalled and drained together, and that the coordinator waits for all of
  them.
- The `#[ignore]`d integration tests — exercise the full sequence end to end
  (requires Redis and PostgreSQL; run with `cargo test -- --ignored`).

### Integration Tests

Run the full integration test (requires Redis and PostgreSQL):

```bash
cargo test test_graceful_shutdown_integration --ignored
```

### Manual Testing

Use the provided test scripts:

**Linux/macOS:**
```bash
./test_shutdown.sh
```

**Windows:**
```powershell
.\test_shutdown.ps1
```

## Implementation Details

### Intended Shutdown Order

The shutdown sequence is designed so that no in-flight request can ever reach a
worker pool that has already been drained. The intended order is:

1. **Stop accepting new HTTP connections** — the HTTP server stops listening and
   refuses new connections.
2. **Drain in-flight requests** — requests already accepted are allowed to
   complete against a fully live worker pool.
3. **Cancel the `ShutdownCoordinator` token and drain background workers** — once
   no requests are in flight, the coordinator broadcasts the shutdown signal and
   each worker completes its current operation or saves state.
4. **Wait for worker completion up to the timeout** — the coordinator waits for
   every `WorkerHandle` to report completion, bounded by the timeout budget
   described below.
5. **Exit** — the process terminates once workers have finished or the timeout
   has expired.

> **Current divergence (tracked by issue #1718):** the implementation in
> `src/shutdown.rs` currently drains workers *inside* the signal future, i.e.
> before the HTTP server stops accepting requests. In-flight requests can
> therefore hit a dead worker pool. The ordering above is the specification the
> fix must satisfy; see [#1718](https://github.com/hman38705/predictiq/issues/1718)
> for the precedent. This document does not change the implementation.

### Timeout Budget

Two independent timeouts bound the shutdown:

- **Global shutdown timeout** — hardcoded to **30 seconds** in
  `src/shutdown.rs`. This is the maximum time the coordinator waits for all
  workers to report completion (step 4 above).
- **`EMAIL_QUEUE_DRAIN_TIMEOUT_SECS`** — a separate budget for the email queue
  worker, defaulting to **60 seconds**, read from the environment in
  `src/shutdown.rs`. It bounds how long the email queue worker may spend
  processing remaining retries and finishing the current job.

When a timeout expires, the coordinator logs an error and the caller force-exits
rather than waiting indefinitely. Worker errors are logged individually and do
not prevent the overall shutdown from completing.

### Shutdown Flow (as currently implemented)

1. Signal handler receives termination signal and signals the main task (it does not drain workers itself)
2. HTTP server stops accepting new connections
3. ShutdownCoordinator broadcasts shutdown signal to all workers (draining begins only after the server has stopped)
4. Each worker:
   - Receives shutdown signal via broadcast channel
   - Completes current operation or saves state
   - Reports completion to coordinator
5. Coordinator waits for all workers with timeout
6. Application exits

This ordering ensures that in-flight requests can still enqueue work while the server is
accepting connections; workers are only drained once the server has stopped accepting new
requests, so no worker receives work after draining starts.

Note that steps 2–4 currently run before step 5, which is the divergence from
the intended order described above (issue #1718).

### Error Handling

- **Timeout**: If workers don't complete within 30 seconds, shutdown proceeds anyway
- **Worker Errors**: Individual worker failures don't prevent overall shutdown
- **Signal Errors**: Signal handler setup failures are logged but don't prevent startup

### State Preservation

- **Blockchain Sync**: Cursor position saved to Redis
- **Email Queue**: Jobs remain in Redis queues for next startup
- **Transaction Monitor**: Watched transactions logged for debugging
- **Rate Limiter**: Final cleanup performed

## Troubleshooting

### Common Issues

1. **Shutdown Timeout**
   - Check worker logs for stuck operations
   - Verify Redis/database connectivity
   - Consider increasing timeout in code

2. **Workers Not Stopping**
   - Ensure broadcast channels are properly subscribed
   - Check for infinite loops without shutdown checks
   - Verify tokio::select! usage

3. **Data Loss**
   - Check that state is saved before worker exit
   - Verify database transactions are committed
   - Ensure Redis operations complete

### Debugging

Enable debug logging to see detailed shutdown flow:

```bash
RUST_LOG=debug cargo run
```

Look for these log messages:
- "Initiating graceful shutdown for N workers"
- "Worker 'name' received shutdown signal"
- "Worker 'name' shutdown complete"
- "All workers completed graceful shutdown"

## Future Improvements

1. **Configurable Timeout**: Make the global shutdown timeout configurable via environment variable
2. **Drain Mode**: Stop accepting new work before shutdown (see the intended order above and issue #1718)
3. **Health Checks**: More detailed worker health reporting
4. **Metrics**: Prometheus metrics for shutdown events
5. **Job Recovery**: Better handling of interrupted jobs on restart

## Dependencies

- `tokio`: Async runtime with signal handling
- `anyhow`: Error handling
- `tracing`: Logging
- `serde_json`: Health endpoint JSON responses

## Compatibility

- **Rust**: 1.70+
- **Platforms**: Linux, macOS, Windows
- **Signals**: SIGTERM, SIGINT (Unix), Ctrl+C/Break/Close/Shutdown (Windows)