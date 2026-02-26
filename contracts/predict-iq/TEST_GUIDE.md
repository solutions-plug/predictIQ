# PredictIQ Test Guide

## Quick Start

```bash
# Run all tests
make test

# Run specific test categories
make test-unit          # Unit tests only
make test-integration   # Integration tests only
make bench              # Gas benchmarks

# Generate coverage report
make coverage

# Run pre-commit checks
make pre-commit
```

## Test Organization

### Unit Tests (`src/modules/*_test.rs`)
Test individual module functions in isolation:
- `admin_test.rs` - Admin and guardian management
- `bets_test.rs` - Betting logic and winnings calculation
- `markets_test.rs` - Market creation and lifecycle
- `circuit_breaker_test.rs` - Emergency pause functionality
- `oracles_test.rs` - Oracle integration and validation

### Integration Tests (`tests/integration_test.rs`)
Test complete workflows across modules:
- Full market lifecycle (create → bet → resolve → claim)
- Multi-market concurrent operations
- Referral system integration
- Conditional market chains
- Emergency pause and recovery
- Governance upgrade workflow

### Benchmark Tests (`benches/gas_benchmark.rs`)
Measure gas costs and performance:
- Market creation with varying outcome counts
- Bet placement performance
- Resolution with multiple bettors
- Metrics calculation overhead

## Running Tests

### Basic Commands

```bash
# All tests
cargo test

# Specific test file
cargo test --test integration_test

# Specific test function
cargo test test_complete_market_lifecycle

# With output
cargo test -- --nocapture

# Single-threaded (for debugging)
cargo test -- --test-threads=1
```

### Advanced Testing

```bash
# Fast parallel execution with nextest
cargo nextest run

# Watch mode (requires cargo-watch)
cargo watch -x test

# Release mode (optimized)
cargo test --release

# With features
cargo test --features testutils
```

## Coverage

### Generate Coverage Report

```bash
# HTML report
cargo llvm-cov --html --open

# Terminal output
cargo llvm-cov

# Check threshold
cargo llvm-cov --fail-under-lines 80
```

### Coverage Goals
- Overall: >80%
- Critical paths: 100%
  - `place_bet`
  - `claim_winnings`
  - `withdraw_refund`
  - `resolve_market`

## Gas Benchmarks

### Running Benchmarks

```bash
# All benchmarks
cargo test --benches

# Specific benchmark
cargo test --bench gas_benchmark bench_create_market_10_outcomes

# With detailed output
cargo test --benches -- --nocapture
```

### Benchmark Targets
- Market creation (10 outcomes): <3M instructions
- Market creation (50 outcomes): <8M instructions
- Market creation (100 outcomes): <15M instructions
- Bet placement: <2M instructions
- Market resolution: <10M instructions

## Writing Tests

### Test Structure

```rust
#[test]
fn test_descriptive_name() {
    // Setup
    let (env, client, admin) = setup();
    
    // Execute
    let result = client.some_operation(&param);
    
    // Assert
    assert_eq!(result, expected_value);
}
```

### Error Testing

```rust
#[test]
fn test_error_condition() {
    let (env, client, admin) = setup();
    
    let result = client.try_operation(&invalid_param);
    assert_eq!(result, Err(Ok(ErrorCode::ExpectedError)));
}
```

### Using Test Helpers

```rust
use crate::common::*;

#[test]
fn test_with_helpers() {
    let (env, client, admin, token) = setup_with_token();
    
    let market_id = create_market(&client, &env, &admin, &token);
    let user = setup_user_with_balance(&env, &token, 10_000);
    
    place_bet_helper(&client, &user, market_id, 0, 1_000, &token);
    resolve_and_verify(&client, market_id, 0);
}
```

## Debugging Tests

### Enable Logging

```bash
# Set log level
RUST_LOG=debug cargo test

# Soroban-specific logging
RUST_LOG=soroban_sdk=debug cargo test
```

### Print Debugging

```rust
#[test]
fn test_with_debug() {
    let result = some_operation();
    println!("Result: {:?}", result);  // Use -- --nocapture to see
    assert!(result.is_ok());
}
```

### Isolate Failing Tests

```bash
# Run only failing test
cargo test test_name -- --exact

# Run with backtrace
RUST_BACKTRACE=1 cargo test test_name
```

## Snapshot Testing

### Update Snapshots

```bash
# Run tests (generates snapshots)
cargo test

# Review changes
git diff test_snapshots/

# Commit if intentional
git add test_snapshots/
```

### Snapshot Best Practices
- Review all snapshot changes carefully
- Don't commit unintended changes
- Use snapshots for complex output validation
- Avoid for frequently changing data

## CI/CD Integration

### GitHub Actions Workflow
Tests run automatically on:
- Push to main/develop
- Pull requests
- Manual workflow dispatch

### CI Checks
1. Unit tests
2. Integration tests
3. Gas benchmarks
4. Code coverage (>80%)
5. Security audit
6. Clippy lints
7. Code formatting
8. Snapshot validation
9. Optimized build

### Local CI Simulation

```bash
# Run all CI checks locally
make ci
```

## Performance Testing

### Profiling

```bash
# Time profiling
cargo test --benches -- --profile-time=5

# Memory profiling (requires valgrind)
make memcheck
```

### Stress Testing

```bash
# Run ignored stress tests
cargo test -- --ignored

# Custom stress test
cargo test --release test_large_market
```

## Common Issues

### Test Failures

**Issue**: Tests pass locally but fail in CI
- **Solution**: Ensure deterministic test data, avoid time-dependent logic

**Issue**: Flaky tests
- **Solution**: Remove arbitrary timeouts, use proper async utilities

**Issue**: Snapshot mismatches
- **Solution**: Review changes, update if intentional

### Performance Issues

**Issue**: Tests run slowly
- **Solution**: Use `cargo nextest` for parallel execution

**Issue**: High memory usage
- **Solution**: Run tests in release mode: `cargo test --release`

### Coverage Issues

**Issue**: Coverage below threshold
- **Solution**: Add tests for uncovered branches

**Issue**: Coverage report not generating
- **Solution**: Install `cargo-llvm-cov`: `cargo install cargo-llvm-cov`

## Best Practices

### Test Naming
- Use descriptive names: `test_place_bet_after_deadline`
- Follow pattern: `test_<action>_<condition>_<expected_result>`

### Test Independence
- Each test should be self-contained
- Don't rely on test execution order
- Clean up state if needed

### Assertions
- Use specific assertions: `assert_eq!` over `assert!`
- Include helpful error messages
- Test both success and failure cases

### Test Data
- Use deterministic values
- Avoid magic numbers, use constants
- Make test data realistic

### Performance
- Keep tests fast (<1s each)
- Use mocking for external dependencies
- Parallelize when possible

## Resources

- [Soroban Testing Docs](https://soroban.stellar.org/docs/how-to-guides/testing)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-nextest](https://nexte.st/)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)

## Support

For test-related questions:
1. Check this guide
2. Review existing tests for examples
3. Check CI logs for detailed errors
4. Open an issue with test failure details
