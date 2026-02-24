# Issue #29: Social Recovery & Guardian Governance - Implementation Summary

## Overview
Implemented social recovery system with guardian governance to eliminate single-point-of-failure of the Admin key through a 3-of-5 multisig recovery mechanism with 72-hour timelock.

## Changes Made

### 1. Guardian Governance Module (`src/modules/guardians.rs`)
- **Constants**:
  - `TIMELOCK_SECONDS = 259200` (72 hours)
  - `REQUIRED_GUARDIANS = 3` (3-of-5 threshold)
  - `TOTAL_GUARDIANS = 5`

- **RecoveryState Structure**:
  ```rust
  pub struct RecoveryState {
      pub new_admin: Address,
      pub approvals: Vec<Address>,
      pub initiated_at: u64,
  }
  ```

- **Functions**:
  - `set_guardians(e: &Env, guardians: Vec<Address>)`: Admin sets 5 guardians
  - `sign_reset_admin(e: &Env, guardian: Address, new_admin: Address)`: Guardian signs recovery
  - `get_recovery_state(e: &Env) -> Option<RecoveryState>`: Query recovery status
  - `is_recovery_active(e: &Env) -> bool`: Check if 3/5 threshold met
  - `finalize_recovery(e: &Env) -> Result<Address, ErrorCode>`: Complete recovery after timelock

### 2. Error Handling (`src/errors.rs`)
Added error codes:
- `InsufficientGuardians = 130`: Less than required guardians
- `RecoveryNotActive = 131`: Recovery not initiated or insufficient approvals
- `RecoveryTimelockNotExpired = 132`: Attempting finalization before 72 hours
- `RecoveryAlreadyActive = 133`: Conflicting recovery attempt

### 3. Main Contract Interface (`src/lib.rs`)
Public functions:
- `set_guardians(e: Env, guardians: Vec<Address>)`: Admin-only guardian setup
- `sign_reset_admin(e: Env, guardian: Address, new_admin: Address)`: Guardian approval
- `get_recovery_state(e: Env) -> Option<RecoveryState>`: Query recovery
- `is_recovery_active(e: Env) -> bool`: Check recovery status
- `finalize_recovery(e: Env) -> Result<Address, ErrorCode>`: Execute recovery

### 4. Integration Tests (`tests/guardian_governance_test.rs`)
Comprehensive test suite covering all scenarios:

#### Test 1: `test_two_of_five_guardians_admin_unchanged` ✅
- **Scenario**: 2 guardians sign recovery
- **Expected**: Admin remains unchanged, recovery not active
- **Result**: PASSED

#### Test 2: `test_three_of_five_guardians_recovery_active` ✅
- **Scenario**: 3 guardians sign recovery
- **Expected**: Recovery active but admin still old
- **Result**: PASSED

#### Test 3: `test_finalize_recovery_fails_before_72_hours` ✅
- **Scenario**: Attempt finalization after 24 hours
- **Expected**: Fails with RecoveryTimelockNotExpired
- **Result**: PASSED

#### Test 4: `test_finalize_recovery_succeeds_after_72_hours` ✅
- **Scenario**: Finalize after 72 hours with 3/5 approvals
- **Expected**: Admin successfully updated
- **Result**: PASSED

#### Test 5: `test_all_five_guardians_sign` ✅
- **Scenario**: All 5 guardians sign and finalize after 72 hours
- **Expected**: Recovery succeeds
- **Result**: PASSED

#### Test 6: `test_guardian_cannot_sign_twice` ✅
- **Scenario**: Guardian attempts to sign twice
- **Expected**: Only counted once
- **Result**: PASSED

## Verification Checklist

✅ **2/5 Guardians sign recovery; verify Admin address remains unchanged**
- Test: `test_two_of_five_guardians_admin_unchanged`
- Status: PASSED

✅ **3/5 Guardians sign; verify recovery_active is true but Admin is still old**
- Test: `test_three_of_five_guardians_recovery_active`
- Status: PASSED

✅ **Attempt to finalize_recovery after 24 hours fails**
- Test: `test_finalize_recovery_fails_before_72_hours`
- Status: PASSED

✅ **finalize_recovery after 72 hours successfully updates the Admin address**
- Test: `test_finalize_recovery_succeeds_after_72_hours`
- Status: PASSED

## Technical Implementation Details

### Recovery Flow
```
1. Admin sets 5 guardians → set_guardians()
2. Admin key lost
3. Guardian 1 signs → sign_reset_admin(new_admin)
4. Guardian 2 signs → sign_reset_admin(new_admin)
5. Guardian 3 signs → sign_reset_admin(new_admin)
   ↓
   Recovery Active (3/5 threshold met)
   ↓
6. Wait 72 hours (timelock)
7. Anyone calls finalize_recovery()
   ↓
   Admin updated to new_admin
```

### Security Features

1. **Threshold Signature**: Requires 3 of 5 guardians (60% consensus)
2. **Timelock**: 72-hour delay prevents hostile takeovers
3. **Duplicate Prevention**: Guardians can't sign twice
4. **Admin Control**: Only admin can set guardians
5. **Transparency**: Recovery state is publicly queryable

### Guardian Selection Best Practices

Recommended guardian distribution:
- 2 guardians: Core team members
- 2 guardians: Trusted community members
- 1 guardian: Third-party security firm or legal entity

Ensures:
- No single entity controls recovery
- Geographic distribution
- Organizational diversity

## Build & Test Results

### Build Status
```bash
cargo build --target wasm32-unknown-unknown --release
✅ SUCCESS - WASM compiled successfully
```

### Test Results
```bash
cargo test --test guardian_governance_test
✅ 6 tests PASSED
   - test_two_of_five_guardians_admin_unchanged
   - test_three_of_five_guardians_recovery_active
   - test_finalize_recovery_fails_before_72_hours
   - test_finalize_recovery_succeeds_after_72_hours
   - test_all_five_guardians_sign
   - test_guardian_cannot_sign_twice
```

## Files Modified/Created

### Created
- `src/modules/guardians.rs` - Guardian governance module
- `tests/guardian_governance_test.rs` - Integration tests

### Modified
- `src/errors.rs` - Added guardian-related error codes
- `src/modules/mod.rs` - Registered guardians module
- `src/lib.rs` - Added guardian governance functions

## Deployment Instructions

### 1. Deploy Contract
```bash
cd contracts/predict-iq
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predict_iq.wasm \
  --network testnet \
  --source $DEPLOYER_SECRET_KEY
```

### 2. Set Guardians
```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn set_guardians \
  --network testnet \
  --source $ADMIN_SECRET_KEY \
  --arg guardians='["GUARDIAN1_ADDRESS", "GUARDIAN2_ADDRESS", "GUARDIAN3_ADDRESS", "GUARDIAN4_ADDRESS", "GUARDIAN5_ADDRESS"]'
```

### 3. Recovery Process (If Admin Key Lost)

**Step 1**: Guardians sign recovery
```bash
# Guardian 1
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn sign_reset_admin \
  --network testnet \
  --source $GUARDIAN1_SECRET_KEY \
  --arg guardian=$GUARDIAN1_ADDRESS \
  --arg new_admin=$NEW_ADMIN_ADDRESS

# Guardian 2
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn sign_reset_admin \
  --network testnet \
  --source $GUARDIAN2_SECRET_KEY \
  --arg guardian=$GUARDIAN2_ADDRESS \
  --arg new_admin=$NEW_ADMIN_ADDRESS

# Guardian 3
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn sign_reset_admin \
  --network testnet \
  --source $GUARDIAN3_SECRET_KEY \
  --arg guardian=$GUARDIAN3_ADDRESS \
  --arg new_admin=$NEW_ADMIN_ADDRESS
```

**Step 2**: Check recovery status
```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn is_recovery_active \
  --network testnet
# Should return: true
```

**Step 3**: Wait 72 hours

**Step 4**: Finalize recovery
```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn finalize_recovery \
  --network testnet \
  --source $ANY_ADDRESS
```

## Security Considerations

### Strengths
1. **No Single Point of Failure**: Requires 3 of 5 guardians
2. **Timelock Protection**: 72-hour delay prevents rushed decisions
3. **Transparent Process**: All recovery attempts are on-chain
4. **Permissionless Finalization**: Anyone can complete after timelock

### Risks & Mitigations
1. **Guardian Collusion**: 
   - Mitigation: Select diverse, independent guardians
   - Mitigation: Geographic and organizational distribution

2. **Guardian Key Loss**:
   - Mitigation: 5 guardians with 3 required (40% redundancy)
   - Mitigation: Guardian key backup procedures

3. **Hostile Takeover Attempt**:
   - Mitigation: 72-hour timelock allows detection
   - Mitigation: Community monitoring of recovery events

4. **Guardian Compromise**:
   - Mitigation: Requires 3 compromised keys (difficult)
   - Mitigation: Regular guardian rotation recommended

## Operational Procedures

### Guardian Management
- **Initial Setup**: Admin sets 5 guardians at deployment
- **Guardian Rotation**: Admin can update guardians periodically
- **Guardian Communication**: Establish secure communication channels
- **Emergency Contacts**: Maintain guardian contact information

### Monitoring
- Monitor `sign_reset_admin` events
- Alert on recovery initiation
- Track timelock expiration
- Notify community of recovery attempts

### Recovery Scenarios
1. **Admin Key Lost**: Follow standard recovery process
2. **Admin Compromised**: Guardians can recover to new secure key
3. **Admin Unavailable**: Recovery after 72-hour timelock

## Future Enhancements

Potential improvements:
- Variable guardian count (e.g., 5-of-9, 7-of-11)
- Configurable timelock duration
- Guardian rotation without admin
- Recovery cancellation mechanism
- Multi-stage recovery with escalating timelocks
- Guardian reputation system

## Branch Information

**Branch**: `features/issue-29-social-recovery-guardian-governance`
**Base Branch**: `develop`
**Status**: Ready for PR

## PR Checklist

- ✅ All tests passing (6/6)
- ✅ WASM builds successfully
- ✅ Verification checklist completed
- ✅ Security considerations documented
- ✅ Deployment instructions provided
- ✅ No breaking changes

---

**Implementation Date**: 2026-02-23
**Developer**: Kiro AI Assistant
**Issue**: #29 - Social Recovery & Guardian Governance
