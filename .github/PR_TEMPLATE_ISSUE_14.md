# Pull Request: Permissioned Creation & Tiered Market Levels

## Issue
Closes #14

## Description
Implements a comprehensive system to prevent spam and reward reputable creators with lower fees through:
- **Creation Deposits**: Lock XLM from creators until market resolution
- **Reputation System**: High-reputation creators bypass deposit requirements
- **Tiered Markets**: Three tiers (Basic, Pro, Institutional) with decreasing commission rates

## Changes

### New Features
1. **MarketTier Enum** - Basic (100%), Pro (75%), Institutional (50%) commission rates
2. **CreatorReputation System** - None, Basic, Pro, Institutional levels
3. **Creation Deposit Mechanism** - Configurable deposit amount locked until resolution
4. **Tiered Fee Calculation** - `calculate_tiered_fee()` applies tier-based discounts
5. **Reputation Management** - Admin functions to manage creator reputations

### API Changes
⚠️ **Breaking Change**: `create_market` signature updated
```rust
// Before
create_market(creator, description, options, deadline, resolution_deadline, oracle_config)

// After
create_market(creator, description, options, deadline, resolution_deadline, oracle_config, tier, native_token)
```

### New Public Functions
- `set_creator_reputation(creator, reputation)`
- `get_creator_reputation(creator)`
- `set_creation_deposit(amount)`
- `get_creation_deposit()`
- `release_creation_deposit(market_id, native_token)`

## Verification Checklist

All requirements from Issue #14 have been implemented and tested:

- ✅ **Market creation fails without sufficient deposit**
  - Test: `test_market_creation_fails_without_deposit`
  - Verifies insufficient balance prevents market creation

- ✅ **Deposit released after market resolution**
  - Test: `test_deposit_released_after_resolution`
  - Confirms deposit can be released once market is resolved

- ✅ **Pro reputation skips deposit**
  - Test: `test_pro_reputation_skips_deposit`
  - Verifies Pro creators don't need to provide deposit

- ✅ **Institutional reputation skips deposit**
  - Test: `test_institutional_reputation_skips_deposit`
  - Verifies Institutional creators don't need to provide deposit

- ✅ **Commission rates decrease with tier**
  - Test: `test_tiered_commission_rates`
  - Confirms Basic (100%), Pro (75%), Institutional (50%) rates

- ✅ **Reputation management works**
  - Test: `test_reputation_management`
  - Validates get/set reputation functions

## Test Results
```
running 11 tests
test test::test_guardian_pause_functionality ... ok
test test::test_market_creation_fails_without_deposit ... ok
test test::test_place_bet_blocked_when_paused ... ok
test test::test_market_creation_with_sufficient_deposit ... ok
test test::test_deposit_released_after_resolution ... ok
test test::test_only_guardian_can_unpause ... ok
test test::test_partial_freeze_claim_winnings_works_when_paused ... ok
test test::test_reputation_management ... ok
test test::test_institutional_reputation_skips_deposit ... ok
test test::test_pro_reputation_skips_deposit ... ok
test test::test_tiered_commission_rates ... ok

test result: ok. 11 passed; 0 failed
```

## Files Changed
- `contracts/predict-iq/src/types.rs` - Added MarketTier, CreatorReputation enums
- `contracts/predict-iq/src/errors.rs` - Added InsufficientDeposit error
- `contracts/predict-iq/src/modules/markets.rs` - Updated create_market, added reputation functions
- `contracts/predict-iq/src/modules/fees.rs` - Added calculate_tiered_fee
- `contracts/predict-iq/src/lib.rs` - Updated public API
- `contracts/predict-iq/src/test.rs` - Added comprehensive tests
- `contracts/predict-iq/benches/gas_benchmark.rs` - Updated benchmarks

## Security Considerations
- ✅ Only admins can set creator reputation (prevents self-promotion)
- ✅ Balance checked before deposit lock
- ✅ Deposits locked in contract until resolution
- ✅ Reputation bypass only for Pro/Institutional levels

## Gas Optimization
- Reputation check before balance check (fail fast)
- Deposit only locked when required
- Single storage write per reputation update

## Migration Guide

### For Contract Integrators
Update all `create_market` calls to include new parameters:

```rust
// Add these parameters
let tier = MarketTier::Basic;
let native_token = Address::from_string("G...");

client.create_market(
    &creator,
    &description,
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config,
    &tier,              // NEW
    &native_token,      // NEW
);
```

### For Admins
New admin functions available:

```rust
// Set creation deposit (in stroops)
client.set_creation_deposit(&10_000_000); // 10 XLM

// Manage creator reputation
client.set_creator_reputation(&creator_address, &CreatorReputation::Pro);

// Check reputation
let rep = client.get_creator_reputation(&creator_address);
```

## Documentation
See `IMPLEMENTATION_ISSUE_14.md` for detailed implementation notes and usage examples.

## Next Steps
- [ ] Deploy to testnet for integration testing
- [ ] Update frontend to support tier selection UI
- [ ] Document admin procedures for reputation management
- [ ] Consider automated reputation upgrades based on market history

## Checklist
- [x] Code follows project style guidelines
- [x] All tests pass
- [x] New tests added for new functionality
- [x] Breaking changes documented
- [x] Security considerations reviewed
- [x] Gas optimization considered
- [x] Documentation updated

## Related Issues
- Implements #14 - Permissioned Creation & Tiered Market Levels

---

**Branch**: `features/issue-14-Permissioned-Creation-Tiered-Market-Levels`  
**Target**: `develop` (or `main` if develop doesn't exist)  
**Type**: Feature  
**Breaking Changes**: Yes - `create_market` signature updated
