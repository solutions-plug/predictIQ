# Quick Reference: Tiered Markets & Reputation System

## For Creators

### Creating a Market

```rust
// Basic tier (standard fees, requires deposit)
let market_id = client.create_market(
    &creator,
    &String::from_str(&env, "Will BTC reach $100k?"),
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config,
    &MarketTier::Basic,
    &native_token,
);

// Pro tier (25% fee discount, may skip deposit if reputation is Pro+)
let market_id = client.create_market(
    &creator,
    &description,
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config,
    &MarketTier::Pro,
    &native_token,
);

// Institutional tier (50% fee discount, may skip deposit if reputation is Institutional)
let market_id = client.create_market(
    &creator,
    &description,
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config,
    &MarketTier::Institutional,
    &native_token,
);
```

### Deposit Requirements

| Reputation Level | Deposit Required? |
|-----------------|-------------------|
| None | ✅ Yes |
| Basic | ✅ Yes |
| Pro | ❌ No |
| Institutional | ❌ No |

## For Admins

### Configure Creation Deposit

```rust
// Set deposit amount (in stroops, 1 XLM = 10,000,000 stroops)
client.set_creation_deposit(&10_000_000); // 10 XLM

// Get current deposit amount
let deposit = client.get_creation_deposit();

// Disable deposits
client.set_creation_deposit(&0);
```

### Manage Creator Reputation

```rust
// Set reputation
client.set_creator_reputation(
    &creator_address,
    &CreatorReputation::Pro
);

// Check reputation
let reputation = client.get_creator_reputation(&creator_address);

// Reputation levels
CreatorReputation::None          // Default, requires deposit
CreatorReputation::Basic         // Requires deposit
CreatorReputation::Pro           // Skips deposit, 25% fee discount
CreatorReputation::Institutional // Skips deposit, 50% fee discount
```

### Release Deposits After Resolution

```rust
// After market is resolved, release creator's deposit
client.release_creation_deposit(&market_id, &native_token);
```

## Fee Structure

### Commission Rates by Tier

Assuming base fee of 100 basis points (1%):

| Tier | Fee Multiplier | Effective Rate | Example (1000 XLM) |
|------|---------------|----------------|-------------------|
| Basic | 100% | 1.00% | 10 XLM |
| Pro | 75% | 0.75% | 7.5 XLM |
| Institutional | 50% | 0.50% | 5 XLM |

### Calculate Fees

```rust
// In fees module
let fee = calculate_tiered_fee(&env, amount, &market.tier);

// Examples:
// Basic tier: 1000 XLM * 1% = 10 XLM
// Pro tier: 1000 XLM * 0.75% = 7.5 XLM
// Institutional tier: 1000 XLM * 0.5% = 5 XLM
```

## Workflow Examples

### New Creator (No Reputation)

1. Creator has 100 XLM
2. Creation deposit set to 10 XLM
3. Creator calls `create_market` with Basic tier
4. Contract locks 10 XLM deposit
5. Market created successfully
6. After resolution, admin calls `release_creation_deposit`
7. Creator receives 10 XLM back

### Trusted Creator (Pro Reputation)

1. Admin sets creator reputation to Pro
2. Creator calls `create_market` with Pro tier
3. No deposit required (bypassed)
4. Market created with 25% fee discount
5. No deposit to release after resolution

### Institutional Creator

1. Admin sets creator reputation to Institutional
2. Creator calls `create_market` with Institutional tier
3. No deposit required (bypassed)
4. Market created with 50% fee discount
5. No deposit to release after resolution

## Error Handling

### InsufficientDeposit Error

```rust
let result = client.try_create_market(...);

match result {
    Ok(market_id) => {
        // Market created successfully
    },
    Err(Ok(ErrorCode::InsufficientDeposit)) => {
        // Creator doesn't have enough XLM for deposit
        // Options:
        // 1. Ask creator to add more XLM
        // 2. Admin upgrades creator reputation
        // 3. Admin reduces deposit requirement
    },
    Err(e) => {
        // Other error
    }
}
```

## Best Practices

### For Platform Operators

1. **Start Conservative**: Set deposit high initially (e.g., 50-100 XLM)
2. **Monitor Spam**: Track market creation patterns
3. **Reward Quality**: Upgrade reputation for creators with successful markets
4. **Adjust Dynamically**: Lower deposit as platform matures
5. **Automate Releases**: Build system to auto-release deposits after resolution

### For Reputation Management

1. **Criteria for Pro**:
   - 5+ successful markets
   - No disputes or cancellations
   - Good community feedback

2. **Criteria for Institutional**:
   - 20+ successful markets
   - Verified organization
   - Significant trading volume
   - Excellent track record

3. **Downgrade Policy**:
   - Consider downgrading for repeated issues
   - Implement appeals process

### For Creators

1. **Start with Basic**: Build reputation over time
2. **Choose Appropriate Tier**: Match tier to market importance
3. **Maintain Quality**: Good markets lead to reputation upgrades
4. **Plan for Deposit**: Ensure sufficient XLM balance

## Integration Checklist

- [ ] Update frontend to show tier selection
- [ ] Display deposit requirements to creators
- [ ] Show fee discounts for each tier
- [ ] Implement reputation badge system
- [ ] Add admin panel for reputation management
- [ ] Create deposit release workflow
- [ ] Add analytics for tier usage
- [ ] Document tier benefits for users

## Support

For questions or issues:
- Check `IMPLEMENTATION_ISSUE_14.md` for detailed implementation notes
- Review test cases in `contracts/predict-iq/src/test.rs`
- See `PR_TEMPLATE_ISSUE_14.md` for migration guide
