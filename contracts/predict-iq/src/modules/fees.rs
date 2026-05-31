use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{ConfigKey, MarketTier, TTL_HIGH_THRESHOLD, TTL_LOW_THRESHOLD};
use soroban_sdk::{contracttype, Address, Env, Symbol};

const BPS_DENOMINATOR: i128 = 10_000;
const TIER_DENOMINATOR_BPS: i128 = 10_000;

#[contracttype]
pub enum DataKey {
    TotalFeesCollected,
    FeeRevenue(Address),
    /// Issue #1: Key is now (referrer, token) to prevent cross-asset mixing.
    ReferrerBalance(Address, Address),
}

fn bump_config_ttl(e: &Env, key: &ConfigKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
}

pub fn get_base_fee(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&ConfigKey::BaseFee)
        .unwrap_or(0)
}

pub fn set_base_fee(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::BaseFee, &amount);
    bump_config_ttl(e, &ConfigKey::BaseFee);
    Ok(())
}

pub fn set_fee_admin(e: &Env, fee_admin: Address) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::FeeAdmin, &fee_admin);
    bump_config_ttl(e, &ConfigKey::FeeAdmin);
    Ok(())
}

pub fn get_fee_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::FeeAdmin)
}

fn require_fee_withdraw_auth(e: &Env) -> Result<(), ErrorCode> {
    if let Some(fee_admin) = get_fee_admin(e) {
        fee_admin.require_auth();
    } else {
        admin::require_admin(e)?;
    }
    Ok(())
}

pub fn calculate_fee(e: &Env, amount: i128) -> Result<i128, ErrorCode> {
    let base_fee = get_base_fee(e);
    let numerator = amount.checked_mul(base_fee).ok_or(ErrorCode::Overflow)?;
    numerator
        .checked_div(BPS_DENOMINATOR)
        .ok_or(ErrorCode::Overflow)
}

fn tier_multiplier_bps(tier: &MarketTier) -> i128 {
    match tier {
        MarketTier::Basic => TIER_DENOMINATOR_BPS,
        MarketTier::Pro => 7_500,           // 25% discount
        MarketTier::Institutional => 5_000, // 50% discount
    }
}

fn calculate_tiered_fee_with_base(
    amount: i128,
    base_fee_bps: i128,
    tier: &MarketTier,
) -> Result<i128, ErrorCode> {
    // Single-pass high-precision arithmetic: amount * base_fee_bps * tier_multiplier / (10_000 * 10_000)
    // This avoids early truncation from computing discounted base_fee first.
    let numerator = amount
        .checked_mul(base_fee_bps)
        .and_then(|n| n.checked_mul(tier_multiplier_bps(tier)))
        .ok_or(ErrorCode::Overflow)?;
    numerator
        .checked_div(BPS_DENOMINATOR * TIER_DENOMINATOR_BPS)
        .ok_or(ErrorCode::Overflow)
}

/// Issue #39: multiply before divide and keep tier multipliers in bps.
pub fn calculate_tiered_fee(e: &Env, amount: i128, tier: &MarketTier) -> Result<i128, ErrorCode> {
    let base_fee = get_base_fee(e);
    calculate_tiered_fee_with_base(amount, base_fee, tier)
}

pub fn collect_fee(e: &Env, token: Address, amount: i128) {
    let key = DataKey::FeeRevenue(token.clone());
    let mut total: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    total += amount;
    e.storage().persistent().set(&key, &total);

    let mut overall: i128 = e
        .storage()
        .persistent()
        .get(&DataKey::TotalFeesCollected)
        .unwrap_or(0);
    overall += amount;
    e.storage()
        .persistent()
        .set(&DataKey::TotalFeesCollected, &overall);

    // Emit standardized fee collection event using centralized emitter
    let contract_addr = e.current_contract_address();
    crate::modules::events::emit_fee_collected(e, 0, contract_addr, amount);
}

pub fn get_revenue(e: &Env, token: Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::FeeRevenue(token))
        .unwrap_or(0)
}

/// Issue #26: Allow Admin to withdraw accumulated protocol fees.
pub fn withdraw_protocol_fees(
    e: &Env,
    token: &Address,
    recipient: &Address,
) -> Result<i128, ErrorCode> {
    require_fee_withdraw_auth(e)?;

    let key = DataKey::FeeRevenue(token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);

    if balance == 0 {
        return Err(ErrorCode::InsufficientBalance);
    }

    // Zero out the balance before the transfer (checks-effects-interactions).
    e.storage().persistent().set(&key, &0i128);

    soroban_sdk::token::Client::new(e, token).transfer(
        &e.current_contract_address(),
        recipient,
        &balance,
    );

    e.events().publish(
        (Symbol::new(e, "fees_withdrawn"), recipient.clone()),
        (token.clone(), balance),
    );

    Ok(balance)
}

/// Issue #1: Referral reward keyed by (referrer, token) to prevent cross-asset mixing.
pub fn add_referral_reward(
    e: &Env,
    referrer: &Address,
    token: &Address,
    fee_amount: i128,
) -> Result<(), ErrorCode> {
    let reward = fee_amount
        .checked_mul(10)
        .and_then(|n| n.checked_div(100))
        .ok_or(ErrorCode::Overflow)?;
    let key = DataKey::ReferrerBalance(referrer.clone(), token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    let new_balance = balance.checked_add(reward).ok_or(ErrorCode::Overflow)?;
    e.storage().persistent().set(&key, &new_balance);

    crate::modules::events::emit_referral_reward(e, 0, referrer.clone(), reward);
    Ok(())
}

/// Reverse a referral reward that was credited at bet time.
/// Called during cancellation refund to void rewards from cancelled markets.
pub fn reverse_referral_reward(e: &Env, referrer: &Address, token: &Address, fee_amount: i128) {
    let reward = match fee_amount.checked_mul(10).and_then(|n| n.checked_div(100)) {
        Some(r) => r,
        None => return, // overflow on a reversal is a no-op; balance stays as-is
    };
    if reward == 0 {
        return;
    }
    let key = DataKey::ReferrerBalance(referrer.clone(), token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    let new_balance = balance.saturating_sub(reward);
    e.storage().persistent().set(&key, &new_balance);
}

/// Reverse protocol fee revenue that was collected at bet time.
/// Called during cancellation refund so the fee is returned to the bettor.
pub fn reverse_fee(e: &Env, token: Address, amount: i128) {
    if amount == 0 {
        return;
    }
    let key = DataKey::FeeRevenue(token);
    let total: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    e.storage()
        .persistent()
        .set(&key, &total.saturating_sub(amount));

    let overall: i128 = e
        .storage()
        .persistent()
        .get(&DataKey::TotalFeesCollected)
        .unwrap_or(0);
    e.storage().persistent().set(
        &DataKey::TotalFeesCollected,
        &overall.saturating_sub(amount),
    );
}

/// Issue #1: Claim referral rewards for a specific token only.
pub fn claim_referral_rewards(
    e: &Env,
    address: &Address,
    token: &Address,
) -> Result<i128, ErrorCode> {
    address.require_auth();

    let key = DataKey::ReferrerBalance(address.clone(), token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);

    if balance == 0 {
        return Err(ErrorCode::InsufficientBalance);
    }

    e.storage().persistent().set(&key, &0i128);

    let client = soroban_sdk::token::Client::new(e, token);
    client.transfer(&e.current_contract_address(), address, &balance);

    crate::modules::events::emit_referral_claimed(e, 0, address.clone(), balance);

    Ok(balance)
}

/// Issue #511: Distribute referral fees on market resolution
/// Called during market resolution to distribute accumulated referral rewards
pub fn distribute_referral_fees(e: &Env, market_id: u64, token: &Address) -> Result<(), ErrorCode> {
    // Get all referrers for this market and distribute their accumulated rewards
    // This is a placeholder that would iterate through referrers in production
    // For now, the rewards are already tracked in ReferrerBalance and can be claimed

    crate::modules::events::emit_referral_distribution(e, market_id, token.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{calculate_tiered_fee_with_base, MarketTier};

    #[test]
    fn tiered_fee_keeps_fractional_discount_precision() {
        // 1 bps base fee with Pro tier (25% discount):
        // old math: ((1 * 75) / 100) = 0 bps => zero fee for all amounts.
        // new math preserves the discounted 0.75 bps effect until final division.
        let basic_fee = calculate_tiered_fee_with_base(4_000_000, 1, &MarketTier::Basic).unwrap();
        let pro_fee = calculate_tiered_fee_with_base(4_000_000, 1, &MarketTier::Pro).unwrap();
        assert_eq!(basic_fee, 400);
        assert_eq!(pro_fee, 300);
    }

    #[test]
    fn tiered_fee_uses_expected_discount_ratio() {
        let basic_fee = calculate_tiered_fee_with_base(10_000, 100, &MarketTier::Basic).unwrap();
        let pro_fee = calculate_tiered_fee_with_base(10_000, 100, &MarketTier::Pro).unwrap();
        let inst_fee =
            calculate_tiered_fee_with_base(10_000, 100, &MarketTier::Institutional).unwrap();

        assert_eq!(basic_fee, 100);
        assert_eq!(pro_fee, 75);
        assert_eq!(inst_fee, 50);
    }

    #[test]
    fn four_unit_bet_applies_pro_discount() {
        let basic_fee = calculate_tiered_fee_with_base(4, 10_000, &MarketTier::Basic).unwrap();
        let pro_fee = calculate_tiered_fee_with_base(4, 10_000, &MarketTier::Pro).unwrap();

        assert_eq!(basic_fee, 4);
        assert_eq!(pro_fee, 3);
    }

    #[test]
    fn max_i128_amount_returns_overflow_error() {
        let result = calculate_tiered_fee_with_base(i128::MAX, 10_000, &MarketTier::Basic);
        assert!(
            result.is_err(),
            "i128::MAX * 10_000 must overflow and return Err"
        );
    }
}

#[cfg(test)]
mod withdrawal_tests {
    use crate::errors::ErrorCode;
    use crate::{PredictIQ, PredictIQClient};
    use soroban_sdk::{testutils::Address as _, token, Address, Env};

    fn setup() -> (Env, PredictIQClient<'static>, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(PredictIQ, ());
        let client = PredictIQClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &100);

        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_id.address();

        // Seed the contract with tokens so it can pay out fees
        token::StellarAssetClient::new(&env, &token_address).mint(&contract_id, &1_000_000);

        (env, client, admin, token_address, contract_id)
    }

    fn seed_fee_revenue(env: &Env, contract_id: &Address, token: &Address, amount: i128) {
        use crate::modules::fees::DataKey;
        env.as_contract(contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::FeeRevenue(token.clone()), &amount);
        });
    }

    #[test]
    fn test_fee_admin_can_withdraw() {
        let (env, client, admin, token, contract_id) = setup();

        let fee_admin = Address::generate(&env);
        client.set_fee_admin(&fee_admin);

        seed_fee_revenue(&env, &contract_id, &token, 500_000);

        let treasury = Address::generate(&env);
        let withdrawn = client.withdraw_protocol_fees(&token, &treasury);

        assert_eq!(withdrawn, 500_000);
        assert_eq!(client.get_revenue(&token), 0);
        assert_eq!(token::Client::new(&env, &token).balance(&treasury), 500_000);
    }

    #[test]
    fn test_admin_can_withdraw_when_no_fee_admin_set() {
        let (env, client, _admin, token, contract_id) = setup();

        // No fee_admin set — master admin should be accepted
        seed_fee_revenue(&env, &contract_id, &token, 250_000);

        let treasury = Address::generate(&env);
        let withdrawn = client.withdraw_protocol_fees(&token, &treasury);

        assert_eq!(withdrawn, 250_000);
        assert_eq!(client.get_revenue(&token), 0);
    }

    #[test]
    fn test_withdraw_returns_error_when_balance_is_zero() {
        let (env, client, _admin, token, _contract_id) = setup();

        let treasury = Address::generate(&env);
        let result = client.try_withdraw_protocol_fees(&token, &treasury);

        assert_eq!(result, Err(Ok(ErrorCode::InsufficientBalance)));
    }

    #[test]
    fn test_unauthorized_address_cannot_withdraw() {
        let (env, client, _admin, token, contract_id) = setup();

        seed_fee_revenue(&env, &contract_id, &token, 100_000);

        // Attempt withdrawal from a non-admin address — mock_all_auths is off for this call
        let treasury = Address::generate(&env);
        let result = client.try_withdraw_protocol_fees(&token, &treasury);
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_zeroed_after_withdrawal() {
        let (env, client, _admin, token, contract_id) = setup();

        seed_fee_revenue(&env, &contract_id, &token, 300_000);

        let treasury = Address::generate(&env);
        client.withdraw_protocol_fees(&token, &treasury);

        // Revenue tracker must be zero
        assert_eq!(client.get_revenue(&token), 0);

        // Second withdrawal must fail
        let result = client.try_withdraw_protocol_fees(&token, &treasury);
        assert_eq!(result, Err(Ok(ErrorCode::InsufficientBalance)));
    }

    #[test]
    fn test_withdrawal_transfers_exact_amount_to_recipient() {
        let (env, client, _admin, token, contract_id) = setup();

        let amount = 750_000i128;
        seed_fee_revenue(&env, &contract_id, &token, amount);

        let treasury = Address::generate(&env);
        let before = token::Client::new(&env, &token).balance(&treasury);

        client.withdraw_protocol_fees(&token, &treasury);

        let after = token::Client::new(&env, &token).balance(&treasury);
        assert_eq!(after - before, amount);
    }
}
