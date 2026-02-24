# KYC/AML Identity Verification - Quick Start Guide

## Overview
The PredictIQ contract now supports optional KYC/AML identity verification through an external identity service. When configured, only verified users can place bets.

## For Contract Administrators

### Setting Up Identity Verification

1. **Deploy Your Identity Service Contract**
   
   Your identity contract must implement:
   ```rust
   pub fn is_verify(e: Env, user: Address) -> bool
   ```

2. **Configure PredictIQ to Use Your Identity Service**
   
   ```bash
   soroban contract invoke \
     --id <PREDICTIQ_CONTRACT_ID> \
     --fn set_identity_contract \
     --network <NETWORK> \
     --source <ADMIN_SECRET_KEY> \
     --arg contract=<IDENTITY_CONTRACT_ID>
   ```

3. **Verify Configuration**
   
   Test with an unverified user - they should receive error code 130 (IdentityVerificationRequired).

### Disabling Identity Verification

To disable verification, you would need to deploy a new contract instance or implement an admin function to clear the identity contract (not currently implemented).

## For Identity Service Providers

### Minimum Contract Interface

Your identity verification contract must implement:

```rust
#[contract]
pub struct IdentityContract;

#[contractimpl]
impl IdentityContract {
    /// Check if a user is verified
    /// Returns true if verified, false otherwise
    pub fn is_verify(e: Env, user: Address) -> bool {
        // Your verification logic here
        // Check against your KYC/AML database
        // Return verification status
    }
}
```

### Implementation Guidelines

1. **Real-time Checks**: The function is called on every bet placement
2. **Performance**: Keep verification checks fast (< 100ms recommended)
3. **Reliability**: Ensure high availability - failed checks block transactions
4. **Security**: Implement proper access controls for verification status updates
5. **Compliance**: Maintain audit logs of verification checks

### Example Implementation

```rust
use soroban_sdk::{contract, contractimpl, Env, Address, Map};

#[contract]
pub struct IdentityContract;

#[contractimpl]
impl IdentityContract {
    /// Admin function to verify a user
    pub fn verify_user(e: Env, admin: Address, user: Address) {
        admin.require_auth();
        // Check admin permissions
        
        let mut verified: Map<Address, bool> = e
            .storage()
            .persistent()
            .get(&symbol_short!("verified"))
            .unwrap_or(Map::new(&e));
        
        verified.set(user, true);
        e.storage().persistent().set(&symbol_short!("verified"), &verified);
    }
    
    /// Admin function to revoke verification
    pub fn revoke_user(e: Env, admin: Address, user: Address) {
        admin.require_auth();
        // Check admin permissions
        
        let mut verified: Map<Address, bool> = e
            .storage()
            .persistent()
            .get(&symbol_short!("verified"))
            .unwrap_or(Map::new(&e));
        
        verified.set(user, false);
        e.storage().persistent().set(&symbol_short!("verified"), &verified);
    }
    
    /// Check verification status (called by PredictIQ)
    pub fn is_verify(e: Env, user: Address) -> bool {
        let verified: Map<Address, bool> = e
            .storage()
            .persistent()
            .get(&symbol_short!("verified"))
            .unwrap_or(Map::new(&e));
        
        verified.get(user).unwrap_or(false)
    }
}
```

## For End Users

### What This Means for You

- **Verified Users**: No change - place bets as normal
- **Unverified Users**: Cannot place bets until verified
- **Verification Process**: Contact the platform administrator for KYC/AML verification

### Error Messages

If you see error code **130 (IdentityVerificationRequired)**:
- You need to complete KYC/AML verification
- Contact the platform administrator
- Provide required identification documents
- Wait for verification approval

## Testing

### Running Tests

```bash
cd contracts/predict-iq
cargo test --test identity_verification_test
```

### Test Coverage

- ✅ Unverified user rejection
- ✅ Verified user acceptance
- ✅ Real-time verification revocation
- ✅ Backward compatibility (no identity service)

## Integration Examples

### JavaScript/TypeScript (Stellar SDK)

```typescript
import { Contract, SorobanRpc } from '@stellar/stellar-sdk';

// Set identity contract (admin only)
async function setIdentityContract(
  predictIqContract: Contract,
  identityContractId: string,
  adminKeypair: Keypair
) {
  const tx = await predictIqContract.call(
    'set_identity_contract',
    identityContractId
  );
  
  tx.sign(adminKeypair);
  return await server.sendTransaction(tx);
}

// Place bet (will check verification automatically)
async function placeBet(
  predictIqContract: Contract,
  marketId: number,
  outcome: number,
  amount: bigint,
  userKeypair: Keypair
) {
  try {
    const tx = await predictIqContract.call(
      'place_bet',
      marketId,
      outcome,
      amount,
      tokenAddress,
      null // no referrer
    );
    
    tx.sign(userKeypair);
    return await server.sendTransaction(tx);
  } catch (error) {
    if (error.code === 130) {
      console.error('User not verified. Please complete KYC/AML verification.');
    }
    throw error;
  }
}
```

### Python (Stellar SDK)

```python
from stellar_sdk import Soroban, Keypair, TransactionBuilder

def set_identity_contract(
    predict_iq_address: str,
    identity_contract_id: str,
    admin_keypair: Keypair,
    server: SorobanServer
):
    """Set identity contract (admin only)"""
    # Build and submit transaction
    pass

def place_bet(
    predict_iq_address: str,
    market_id: int,
    outcome: int,
    amount: int,
    user_keypair: Keypair,
    server: SorobanServer
):
    """Place bet (will check verification automatically)"""
    try:
        # Build and submit transaction
        pass
    except Exception as e:
        if hasattr(e, 'code') and e.code == 130:
            print("User not verified. Please complete KYC/AML verification.")
        raise
```

## Security Considerations

### For Administrators
- Store admin keys securely (hardware wallet recommended)
- Use multisig for identity contract configuration
- Monitor verification service uptime
- Implement rate limiting on verification checks
- Maintain audit logs

### For Identity Providers
- Implement proper access controls
- Use secure storage for verification data
- Comply with data protection regulations (GDPR, etc.)
- Implement verification expiry/renewal
- Provide audit trails

### For Users
- Protect your private keys
- Complete verification through official channels only
- Be aware of phishing attempts
- Verify contract addresses before interacting

## Troubleshooting

### Common Issues

**Error 130: IdentityVerificationRequired**
- Solution: Complete KYC/AML verification with platform administrator

**Identity Contract Not Responding**
- Check identity service uptime
- Verify contract address is correct
- Check network connectivity

**Verification Not Taking Effect**
- Verification is checked in real-time
- No caching - status updates are immediate
- Ensure identity contract is properly configured

## Support

For technical support or questions:
- Review the [Implementation Summary](./IMPLEMENTATION_ISSUE_27.md)
- Check the [API Documentation](./docs/api/API_DOCUMENTATION.md)
- Review test cases in `contracts/predict-iq/tests/identity_verification_test.rs`

---

**Last Updated**: 2026-02-23
**Feature**: Issue #27 - Institutional KYC/AML Identity Hooks
