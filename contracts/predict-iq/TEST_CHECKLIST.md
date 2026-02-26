# PredictIQ Testing Checklist

## Pre-Commit Checklist

Before committing code, ensure:

- [ ] All tests pass: `make test`
- [ ] Code is formatted: `make format`
- [ ] No clippy warnings: `make lint`
- [ ] Coverage >80%: `make coverage-check`
- [ ] No security vulnerabilities: `make audit`
- [ ] Snapshots reviewed if changed
- [ ] New tests added for new features
- [ ] Documentation updated

## Test Coverage Checklist

### Unit Tests

#### Admin Module
- [ ] Set and get admin
- [ ] Require admin authorization
- [ ] Set and get guardian
- [ ] Require guardian authorization
- [ ] Admin/guardian independence

#### Bets Module
- [ ] Place bet success
- [ ] Place bet with zero/negative amount
- [ ] Place bet with invalid outcome
- [ ] Place bet after deadline
- [ ] Place bet on resolved market
- [ ] Multiple bets same outcome
- [ ] Bets on different outcomes
- [ ] Claim winnings success
- [ ] Claim winnings losing bet
- [ ] Claim winnings before resolution
- [ ] Claim winnings twice (double claim)
- [ ] Claim with no bet placed
- [ ] Winnings calculation single winner
- [ ] Winnings calculation multiple winners
- [ ] Referral rewards tracking
- [ ] Self-referral rejection

#### Markets Module
- [ ] Create basic market
- [ ] Create with single option (fail)
- [ ] Create with too many outcomes (fail)
- [ ] Create with deadline in past (fail)
- [ ] Create with invalid deadline order (fail)
- [ ] Market ID increments
- [ ] Get nonexistent market
- [ ] Creator reputation default
- [ ] Set creator reputation
- [ ] Creation deposit default
- [ ] Set creation deposit
- [ ] Different market tiers
- [ ] Prune before grace period (fail)
- [ ] Prune after grace period
- [ ] Prune unresolved market (fail)

#### Circuit Breaker Module
- [ ] Initial state closed
- [ ] Pause contract
- [ ] Unpause contract
- [ ] Pause blocks operations
- [ ] Unpause allows operations
- [ ] Multiple pause/unpause cycles
- [ ] Set different circuit breaker states
- [ ] Require closed when open
- [ ] Half-open limited operations

#### Oracles Module
- [ ] Validate fresh price
- [ ] Reject stale price
- [ ] Reject low confidence price
- [ ] Oracle result setting
- [ ] Manual resolution

### Integration Tests

- [ ] Complete market lifecycle
- [ ] Multi-market concurrent operations
- [ ] Referral system integration
- [ ] Conditional market chain
- [ ] Emergency pause and recovery
- [ ] Governance upgrade workflow
- [ ] Market cancellation and refunds
- [ ] Fee collection and distribution
- [ ] Reputation-based deposit waiver

### Gas Benchmarks

- [ ] Create market 10 outcomes (<3M instructions)
- [ ] Create market 50 outcomes (<8M instructions)
- [ ] Create market 100 outcomes (<15M instructions)
- [ ] Place multiple bets (<2M per bet)
- [ ] Resolve market (<10M instructions)
- [ ] Get resolution metrics
- [ ] Reject excessive outcomes
- [ ] Full market lifecycle

### Edge Cases

- [ ] Zero amounts
- [ ] Maximum values (u64::MAX, i128::MAX)
- [ ] Empty collections
- [ ] Single participant markets
- [ ] Expired deadlines
- [ ] Stale oracle data
- [ ] Paused contract operations
- [ ] Concurrent operations
- [ ] State transitions
- [ ] Boundary conditions

### Security Tests

- [ ] Authorization checks
- [ ] Reentrancy protection
- [ ] Integer overflow/underflow
- [ ] Double claim prevention
- [ ] Invalid state transitions
- [ ] Access control enforcement
- [ ] Circuit breaker functionality
- [ ] Deposit/withdrawal safety

### Governance Tests

- [ ] Initialize guardians
- [ ] Initialize guardians twice (fail)
- [ ] Add guardian
- [ ] Remove guardian
- [ ] Initiate upgrade
- [ ] Vote for upgrade
- [ ] Execute upgrade before timelock (fail)
- [ ] Execute upgrade after timelock
- [ ] Insufficient votes (fail)
- [ ] Majority vote required
- [ ] Cannot vote twice
- [ ] Only guardians can vote
- [ ] Get upgrade votes
- [ ] Persistent state on upgrade

### Conditional Markets Tests

- [ ] Create conditional market parent not resolved (fail)
- [ ] Create conditional market wrong outcome (fail)
- [ ] Create conditional market success
- [ ] Place bet on conditional market
- [ ] Independent market has no parent
- [ ] Multi-level conditional markets
- [ ] Invalid parent outcome index (fail)

### Cancellation Tests

- [ ] Admin cancel market
- [ ] Withdraw refund full amount
- [ ] Refund no fee collected
- [ ] Refund only on cancelled market (fail)
- [ ] Refund only once (fail)

## Performance Checklist

- [ ] Tests run in <5 minutes total
- [ ] No flaky tests
- [ ] Deterministic test data
- [ ] Proper async utilities used
- [ ] No arbitrary timeouts
- [ ] Parallel execution works
- [ ] Memory usage reasonable

## CI/CD Checklist

- [ ] All CI jobs pass
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Gas benchmarks pass
- [ ] Coverage >80%
- [ ] Security audit clean
- [ ] Clippy warnings resolved
- [ ] Code formatted
- [ ] Snapshots validated
- [ ] Optimized build succeeds
- [ ] WASM size within limits

## Documentation Checklist

- [ ] Test guide updated
- [ ] New test patterns documented
- [ ] Complex tests have comments
- [ ] Test data choices explained
- [ ] Non-obvious assertions documented
- [ ] README updated if needed

## Release Checklist

Before releasing:

- [ ] Full test suite passes
- [ ] Coverage >85%
- [ ] All benchmarks within targets
- [ ] Security audit completed
- [ ] No known bugs
- [ ] Documentation complete
- [ ] Changelog updated
- [ ] Version bumped
- [ ] Git tags created

## Regression Testing

After major changes:

- [ ] Run full test suite
- [ ] Check gas benchmarks
- [ ] Verify coverage maintained
- [ ] Test on testnet
- [ ] Verify state migrations
- [ ] Check backward compatibility

## Test Maintenance

Monthly:

- [ ] Review test coverage
- [ ] Update deprecated patterns
- [ ] Refactor duplicate tests
- [ ] Update test utilities
- [ ] Review flaky tests
- [ ] Update dependencies

## Quality Metrics

Target metrics:

- [ ] Test coverage: >80% (>85% goal)
- [ ] Critical path coverage: 100%
- [ ] Test execution time: <5 minutes
- [ ] Flaky test rate: 0%
- [ ] Bug escape rate: <5%
- [ ] Time to fix failing tests: <1 day

## Sign-Off

Before merging:

- [ ] Developer: All tests pass locally
- [ ] Reviewer: Code review complete
- [ ] CI: All checks pass
- [ ] QA: Manual testing complete (if applicable)
- [ ] Security: No vulnerabilities found
- [ ] Documentation: Updated and reviewed

---

**Last Updated**: [Date]
**Reviewed By**: [Name]
**Next Review**: [Date]
