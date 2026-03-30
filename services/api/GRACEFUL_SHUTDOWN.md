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

### Shutdown Flow

1. Signal handler receives termination signal
2. ShutdownCoordinator broadcasts shutdown signal to all workers
3. Each worker:
   - Receives shutdown signal via broadcast channel
   - Completes current operation or saves state
   - Reports completion to coordinator
4. Coordinator waits for all workers with timeout
5. HTTP server stops accepting new connections
6. Application exits

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

1. **Configurable Timeout**: Make shutdown timeout configurable via environment variable
2. **Drain Mode**: Stop accepting new work before shutdown
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