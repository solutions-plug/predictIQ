# Test script for graceful shutdown behavior on Windows
# This script starts the API server and tests that it shuts down gracefully

$ErrorActionPreference = "Stop"

Write-Host "Starting graceful shutdown test..."

# Set test environment variables
$env:RUST_LOG = "info"
$env:API_BIND_ADDR = "127.0.0.1:8081"
$env:REDIS_URL = "redis://127.0.0.1:6379"
$env:DATABASE_URL = "postgres://postgres:postgres@127.0.0.1/predictiq_test"

try {
    # Start the server in the background
    Write-Host "Starting API server..."
    $serverProcess = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "predictiq-api" -PassThru -NoNewWindow
    
    # Give the server time to start
    Start-Sleep -Seconds 3
    
    # Check if server is running
    if ($serverProcess.HasExited) {
        Write-Host "ERROR: Server failed to start"
        exit 1
    }
    
    Write-Host "Server started with PID: $($serverProcess.Id)"
    
    # Test health endpoint
    Write-Host "Testing health endpoint..."
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:8081/health" -TimeoutSec 5
        Write-Host "Health check passed"
    }
    catch {
        Write-Host "WARNING: Health check failed, but continuing test"
    }
    
    # Send termination signal (Ctrl+C equivalent on Windows)
    Write-Host "Sending termination signal..."
    $serverProcess.CloseMainWindow()
    
    # Wait for graceful shutdown with timeout
    Write-Host "Waiting for graceful shutdown (max 35 seconds)..."
    $timeout = 35
    $elapsed = 0
    
    while (-not $serverProcess.HasExited -and $elapsed -lt $timeout) {
        Start-Sleep -Seconds 1
        $elapsed++
    }
    
    if ($serverProcess.HasExited) {
        Write-Host "SUCCESS: Server shut down gracefully in $elapsed seconds"
    }
    else {
        Write-Host "ERROR: Server did not shut down within timeout, forcing termination"
        $serverProcess.Kill()
        exit 1
    }
    
    Write-Host "Graceful shutdown test completed successfully!"
}
catch {
    Write-Host "ERROR: Test failed with exception: $_"
    if ($serverProcess -and -not $serverProcess.HasExited) {
        $serverProcess.Kill()
    }
    exit 1
}
finally {
    # Cleanup
    if ($serverProcess -and -not $serverProcess.HasExited) {
        $serverProcess.Kill()
    }
}