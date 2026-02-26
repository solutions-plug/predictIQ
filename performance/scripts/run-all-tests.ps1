# Performance Test Runner (PowerShell)
# Runs all performance tests and generates reports

param(
    [string]$ApiUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Starting Performance Test Suite" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan

$ReportsDir = "backend/reports"

# Create reports directory
New-Item -ItemType Directory -Force -Path $ReportsDir | Out-Null

# Check if API is running
Write-Host "`n🔍 Checking API availability at $ApiUrl..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$ApiUrl/health" -Method Get -TimeoutSec 5
    Write-Host "✓ API is running" -ForegroundColor Green
} catch {
    Write-Host "✗ API is not responding at $ApiUrl" -ForegroundColor Red
    Write-Host "Please start the API server first" -ForegroundColor Red
    exit 1
}

# Function to run test
function Run-Test {
    param(
        [string]$TestName,
        [string]$TestFile
    )
    
    Write-Host "`n📊 Running $TestName..." -ForegroundColor Yellow
    Write-Host "-----------------------------------" -ForegroundColor Gray
    
    $env:API_URL = $ApiUrl
    $outputFile = "$ReportsDir/$TestName-raw.json"
    
    try {
        k6 run --out "json=$outputFile" $TestFile
        Write-Host "✓ $TestName completed successfully" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "✗ $TestName failed: $_" -ForegroundColor Red
        return $false
    }
}

# Run tests
$failedTests = 0

if (-not (Run-Test "smoke-test" "backend/k6/smoke-test.js")) { $failedTests++ }
if (-not (Run-Test "load-test" "backend/k6/load-test.js")) { $failedTests++ }
if (-not (Run-Test "stress-test" "backend/k6/stress-test.js")) { $failedTests++ }
if (-not (Run-Test "spike-test" "backend/k6/spike-test.js")) { $failedTests++ }
if (-not (Run-Test "rate-limit-test" "backend/k6/rate-limit-test.js")) { $failedTests++ }
if (-not (Run-Test "cache-test" "backend/k6/cache-test.js")) { $failedTests++ }

# Summary
Write-Host "`n==================================" -ForegroundColor Cyan
Write-Host "📈 Performance Test Summary" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan

if ($failedTests -eq 0) {
    Write-Host "✓ All tests passed!" -ForegroundColor Green
    Write-Host "`nReports generated in: $ReportsDir" -ForegroundColor Cyan
    exit 0
} else {
    Write-Host "✗ $failedTests test(s) failed" -ForegroundColor Red
    Write-Host "`nCheck reports in: $ReportsDir" -ForegroundColor Cyan
    exit 1
}
