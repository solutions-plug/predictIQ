# Comprehensive Testing Strategy for PredictIQ Soroban Contract

## Overview
This document outlines the comprehensive testing strategy for the PredictIQ prediction market smart contract, covering unit tests, integration tests, property-based tests, gas benchmarks, and CI/CD integration.

## Test Coverage Goals
- **Target**: >80% code coverage across all modules
- **Critical paths**: 100% coverage for financial operations (bets, claims, refunds)
- **Edge cases**: Comprehensive coverage of error conditions and boundary cases

## Test Categories

### 1. Unit Tests
**Location**: `src/test*.rs`, `src/modules/*_test.rs`

**Coverage Areas**:
- Individual function behavior
- Error handling and validation
- State transitions
- Access control
- Boundary conditions

**Best Practices**:
- One assertion per test when possible
- Clear test names describing scenario and expected outcome
- Use `try_*` methods to test error cases explicitly
- Mock all external dependencies (tokens, oracles)
- Deterministic test data (no random values)

### 2. Integration Tests
**Location**: `src/test.rs`

**Coverage Areas**:
- Full market lifecycle (create → bet → resolve → claim)
- Multi-user interactions
- Cross-module functionality
- State consistency across operations
- Token transfers and balance updates

**Test Scenarios**:
- Multiple bettors on same market
- Concurrent operations
- Market state machine transitions
- Conditional/chained markets
- Governance workflows

### 3. Property-Based Tests
**Recommended**: Add `proptest` or `quickcheck` for fuzzing

**Properties to Test**:
- Total pool invariants (sum of bets = total pool)
- Winnings calculation correctness
- Fee calculations always positive
- No funds lost or created
- State transitions always valid
- Timestamps monotonic

### 4. Gas Benchmarks
**Location**: `benches/gas_benchmark.rs`

**Metrics**:
- Instruction count per operation
- Memory usage patterns
- Storage access costs
- Worst-case scenarios (max outcomes, max bettors)

**Optimization Targets**:
- Market creation: < 5M instructions
- Bet placement: < 2M instructions
- Resolution: < 10M instructions
- Claim winnings: < 3M instructions

### 5. Security Tests

**Critical Areas**:
- Reentrancy protection
- Integer overflow/underflow
- Authorization checks
- Circuit breaker functionality
- Deposit/withdrawal safety

**Test Patterns**:
```rust
// Authorization
#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_action() { ... }

// Overflow protection
#[test]
fn test_large_bet_amounts() { ... }

// Reentrancy
#[test]
fn test_double_claim_prevented() { ... }
```

### 6. State Machine Tests

**Market States**: Active → Resolved/Cancelled/Disputed
**Bet States**: Placed → Claimed/Refunded

**Validation**:
- All valid transitions work
- Invalid transitions fail with correct error
- State consistency maintained
- No orphaned states

### 7. Edge Case Tests

**Scenarios**:
- Zero amounts
- Maximum values (u64::MAX, i128::MAX)
- Empty collections
- Single participant markets
- Expired deadlines
- Stale oracle data
- Paused contract operations

## Test Organization

```
contracts/predict-iq/
├── src/
│   ├── test.rs                          # Main integration tests
│   ├── test_cancellation.rs             # Cancellation flow tests
│   ├── test_classic_assets.rs           # Asset handling tests
│   ├── test_multi_token.rs              # Multi-token support tests
│   ├── test_referral.rs                 # Referral system tests
│   ├── test_resolution_state_machine.rs # Resolution state tests
│   ├── test_snapshot_voting.rs          # Voting mechanism tests
│   └── modules/
│       ├── admin_test.rs                # Admin function tests
│       ├── bets_test.rs                 # Betting logic tests
│       ├── markets_test.rs              # Market creation tests
│       ├── oracles_test.rs              # Oracle integration tests
│       └── ...
├── benches/
│   └── gas_benchmark.rs                 # Performance benchmarks
└── tests/                               # End-to-end tests (to add)
    └── integration_test.rs
```

## Test Utilities

### Setup Helpers
```rust
fn setup_test_env() -> (Env, Address, PredictIQClient) {
    let e = Env::default();
    e.mock_all_auths();
    // ... initialization
}

fn create_test_market(...) -> u64 { ... }
fn setup_token_with_balance(...) { ... }
fn advance_time(env: &Env, seconds: u64) { ... }
```

### Assertion Helpers
```rust
fn assert_market_status(market: &Market, expected: MarketStatus) { ... }
fn assert_balance_change(before: i128, after: i128, expected_delta: i128) { ... }
fn assert_error_code(result: Result<_, _>, expected: ErrorCode) { ... }
```

## CI/CD Integration

### GitHub Actions Workflow
```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: stellar/binaries@v1
      
      - name: Run unit tests
        run: cargo test --lib
      
      - name: Run integration tests
        run: cargo test --test '*'
      
      - name: Run benchmarks
        run: cargo test --benches
      
      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --output-dir coverage
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/cobertura.xml
```

### Coverage Requirements
- Minimum 80% overall coverage
- 100% coverage for critical paths:
  - `place_bet`
  - `claim_winnings`
  - `withdraw_refund`
  - `resolve_market`
  - Authorization checks

### Performance Regression Detection
- Track instruction counts over time
- Fail CI if operations exceed thresholds
- Monitor storage growth patterns

## Test Data Management

### Deterministic Test Data
```rust
// Good: Deterministic
let timestamp = 1000u64;
let amount = 1_000_000i128;

// Bad: Non-deterministic
let timestamp = env.ledger().timestamp(); // Changes
let amount = random(); // Non-reproducible
```

### Test Fixtures
```rust
struct TestFixture {
    env: Env,
    client: PredictIQClient,
    admin: Address,
    users: Vec<Address>,
    tokens: Vec<Address>,
}

impl TestFixture {
    fn new() -> Self { ... }
    fn create_market(&self, ...) -> u64 { ... }
    fn place_bet(&self, ...) { ... }
}
```

## Snapshot Testing

**Current Usage**: Test snapshots in `test_snapshots/`

**Purpose**:
- Verify contract output consistency
- Detect unintended changes
- Document expected behavior

**Best Practices**:
- Review snapshot changes carefully
- Update snapshots only when intentional
- Don't overuse for frequently changing data

## Error Testing Patterns

### Explicit Error Checking
```rust
#[test]
fn test_bet_after_deadline() {
    let result = client.try_place_bet(...);
    assert_eq!(result, Err(Ok(ErrorCode::MarketClosed)));
}
```

### Should Panic (Use Sparingly)
```rust
#[test]
#[should_panic(expected = "Error(Contract, #105)")]
fn test_unauthorized_admin_action() {
    client.set_admin(&unauthorized_user);
}
```

## Mock Strategies

### Token Mocking
```rust
// Use Stellar test tokens
let token_admin = Address::generate(&env);
let token_id = env.register_stellar_asset_contract_v2(token_admin);
let token_client = token::StellarAssetClient::new(&env, &token_id.address());
token_client.mint(&user, &10_000);
```

### Oracle Mocking
```rust
// Mock oracle responses
let oracle_config = OracleConfig {
    oracle_address: Address::generate(&env),
    feed_id: String::from_str(&env, "test_feed"),
    min_responses: Some(1),
};
```

### Time Mocking
```rust
env.ledger().with_mut(|li| {
    li.timestamp = 1000;
});
```

## Test Maintenance

### Regular Reviews
- Weekly: Review failing tests
- Monthly: Update test coverage reports
- Quarterly: Refactor test utilities
- Per release: Full regression suite

### Documentation
- Comment complex test scenarios
- Document test data choices
- Explain non-obvious assertions
- Link to relevant issues/specs

### Refactoring
- Extract common setup code
- Remove duplicate tests
- Consolidate similar test cases
- Update deprecated patterns

## Performance Testing

### Benchmark Targets
```rust
#[test]
fn bench_create_market_10_outcomes() {
    // Target: < 3M instructions
}

#[test]
fn bench_place_bet() {
    // Target: < 2M instructions
}

#[test]
fn bench_resolve_market_100_bettors() {
    // Target: < 15M instructions
}
```

### Load Testing
- Maximum outcomes per market (100)
- Maximum bets per market (1000+)
- Maximum concurrent operations
- Storage limits

## Continuous Improvement

### Metrics to Track
- Test execution time
- Coverage percentage
- Flaky test rate
- Bug escape rate
- Time to fix failing tests

### Goals
- Zero flaky tests
- < 5 minute test suite execution
- > 85% coverage by Q2 2026
- 100% critical path coverage

## Tools and Dependencies

### Required
- `soroban-sdk` with `testutils` feature
- `cargo test` for test execution
- `cargo-tarpaulin` for coverage

### Recommended
- `proptest` for property-based testing
- `criterion` for detailed benchmarks
- `cargo-nextest` for faster test execution
- `cargo-watch` for continuous testing

## Conclusion

This comprehensive testing strategy ensures the PredictIQ contract is:
- **Reliable**: Extensive test coverage catches bugs early
- **Secure**: Security-focused tests prevent vulnerabilities
- **Performant**: Benchmarks ensure gas efficiency
- **Maintainable**: Well-organized tests are easy to update
- **Documented**: Tests serve as living documentation

All tests must pass before merging to main, and coverage must not decrease.
