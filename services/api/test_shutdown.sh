#!/bin/bash

# Test script for graceful shutdown behavior
# This script starts the API server and tests that it shuts down gracefully

set -e

echo "Starting graceful shutdown test..."

# Set test environment variables
export RUST_LOG=info
export API_BIND_ADDR=127.0.0.1:8081
export REDIS_URL=redis://127.0.0.1:6379
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1/predictiq_test

# Start the server in the background
echo "Starting API server..."
cargo run --bin predictiq-api &
SERVER_PID=$!

# Give the server time to start
sleep 3

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "ERROR: Server failed to start"
    exit 1
fi

echo "Server started with PID: $SERVER_PID"

# Test health endpoint
echo "Testing health endpoint..."
if curl -f http://127.0.0.1:8081/health > /dev/null 2>&1; then
    echo "Health check passed"
else
    echo "WARNING: Health check failed, but continuing test"
fi

# Send SIGTERM to initiate graceful shutdown
echo "Sending SIGTERM signal..."
kill -TERM $SERVER_PID

# Wait for graceful shutdown with timeout
echo "Waiting for graceful shutdown (max 35 seconds)..."
TIMEOUT=35
ELAPSED=0

while kill -0 $SERVER_PID 2>/dev/null; do
    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo "ERROR: Server did not shut down within timeout"
        kill -KILL $SERVER_PID
        exit 1
    fi
    sleep 1
    ELAPSED=$((ELAPSED + 1))
done

echo "SUCCESS: Server shut down gracefully in ${ELAPSED} seconds"

# Verify server is actually stopped
if kill -0 $SERVER_PID 2>/dev/null; then
    echo "ERROR: Server is still running"
    kill -KILL $SERVER_PID
    exit 1
fi

echo "Graceful shutdown test completed successfully!"