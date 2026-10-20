use crate::errors::ErrorCode;
use crate::modules::{admin, circuit_breaker};
use crate::types::{ConfigKey, MarketTier, TTL_HIGH_THRESHOLD, TTL_LOW_THRESHOLD};
use soroban_sdk::{contracttype, Address, Env, Symbol};

const BPS_DENOMINATOR: i128 = 10_000;
const TIER_DENOMINATOR_BPS: i128 = 10_000;

/// Issue #1687: Referral share of the fee, expressed in basis points (10%).
pub const REFERRAL_SHARE_BPS: i128 = 1_000;

#[contracttype]
pub enum DataKey {
    TotalFeesCollected,
    FeeRevenue(Address),
    /// Issue #1684: Fees tracked per (market, token) so they can only be
    /// withdrawn once the market is resolved.
    MarketFeeRevenue(u64, Address),
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

/// Issue #1682: base fee must be a valid bps value in `[0, BPS_DENOMINATOR]`.
/// A negative fee would inflate `net_amount` (minting phantom stake) and a fee
/// above 100% would make `net_amount` negative.
pub fn validate_base_fee(amount: i128) -> Result<(), ErrorCode> {
    if amount < 0 || amount > BPS_DENOMINATOR {
        return Err(ErrorCode::FeeTooHigh);
    }
    Ok(())
}

pub fn set_base_fee(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    validate_base_fee(amount)?;
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
    crate::modules::events::emit_fee_admin_set(e, fee_admin);
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

pub fn collect_fee(e: &Env, token: Address, amount: i128) -> Result<(), ErrorCode> {
    collect_fee_for_market(e, 0, token, amount)
}

/// Issue #1684: record collected fees against the originating market so that
/// they can only be withdrawn once that market is resolved.
pub fn collect_fee_for_market(
    e: &Env,
    market_id: u64,
    token: Address,
    amount: i128,
) -> Result<(), ErrorCode> {
    let key = DataKey::FeeRevenue(token.clone());
    let total: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    let new_total = total.checked_add(amount).ok_or(ErrorCode::ArithmeticOverflow)?;
    e.storage().persistent().set(&key, &new_total);

    let market_key = DataKey::MarketFeeRevenue(market_id, token.clone());
    let market_total: i128 = e.storage().persistent().get(&market_key).unwrap_or(0);
    let new_market_total = market_total
        .checked_add(amount)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    e.storage().persistent().set(&market_key, &new_market_total);

    let overall: i128 = e
        .storage()
        .persistent()
        .get(&DataKey::TotalFeesCollected)
        .unwrap_or(0);
    let new_overall = overall.checked_add(amount).ok_or(ErrorCode::ArithmeticOverflow)?;
    e.storage()
        .persistent()
        .set(&DataKey::TotalFeesCollected, &new_overall);

    // Emit standardized fee collection event using centralized emitter.
    // Fees are held by this contract until withdrawn, so the contract itself
    // is the actual recipient; the token identifies which asset was collected.
    let contract_addr = e.current_contract_address();
    crate::modules::events::emit_fee_collected(e, market_id, token, contract_addr, amount);
    Ok(())
}

/// Issue #1684: fees collected for a specific market and token.
pub fn get_market_revenue(e: &Env, market_id: u64, token: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::MarketFeeRevenue(market_id, token.clone()))
        .unwrap_or(0)
}

pub fn get_revenue(e: &Env, token: Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::FeeRevenue(token))
        .unwrap_or(0)
}

/// Issue #26: Allow Admin to withdraw accumulated protocol fees.
/// Issue #1684: only fees belonging to resolved markets are withdrawable;
/// fees for open markets are held back so cancellation refunds stay funded.
pub fn withdraw_protocol_fees(
    e: &Env,
    token: &Address,
    recipient: &Address,
) -> Result<i128, ErrorCode> {
    // Issue #1685: token egress must respect the emergency pause.
    circuit_breaker::require_not_paused_for_high_risk(e)?;
    require_fee_withdraw_auth(e)?;

    let key = DataKey::FeeRevenue(token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);

    if balance == 0 {
        return Err(ErrorCode::InsufficientBalance);
    }

    // Sum the fees that belong to markets which are no longer open. Fees for
    // open markets remain in the contract to fund potential cancellation
    // refunds (refund = net + fee_paid).
    let withdrawable = withdrawable_fees(e, token)?;

    if withdrawable <= 0 {
        return Err(ErrorCode::InsufficientBalance);
    }

    // Zero out the balance before the transfer (checks-effects-interactions).
    e.storage().persistent().set(&key, &0i128);

    soroban_sdk::token::Client::new(e, token).transfer(
        &e.current_contract_address(),
        recipient,
        &withdrawable,
    );

    e.events().publish(
        (Symbol::new(e, "fees_withdrawn"), recipient.clone()),
        (token.clone(), withdrawable),
    );

    Ok(withdrawable)
}

/// Issue #1684: total fees for `token` that are safe to withdraw, i.e. those
/// belonging to markets that are resolved (or otherwise closed). Fees for
/// markets still accepting bets are held back.
fn withdrawable_fees(e: &Env, token: &Address) -> Result<i128, ErrorCode> {
    let total = get_revenue(e, token.clone());
    let open = open_market_fees(e, token);
    Ok(total.saturating_sub(open))
}

/// Sum of fees held for markets that are still open (not resolved/cancelled).
fn open_market_fees(e: &Env, token: &Address) -> i128 {
    let mut sum: i128 = 0;
    for market_id in crate::modules::markets::open_market_ids(e) {
        sum = sum.saturating_add(get_market_revenue(e, market_id, token));
    }
    sum
}

/// Issue #1: Referral reward keyed by (referrer, token) to prevent cross-asset mixing.
///
/// Issue #1687: Returns the exact reward credited so callers can persist it and
/// later reverse precisely that amount (avoiding aggregate-vs-per-bet rounding drift).
pub fn add_referral_reward(
    e: &Env,
    referrer: &Address,
    token: &Address,
    fee_amount: i128,
) -> Result<i128, ErrorCode> {
    let reward = fee_amount
        .checked_mul(REFERRAL_SHARE_BPS)
        .and_then(|n| n.checked_div(BPS_DENOMINATOR))
        .ok_or(ErrorCode::Overflow)?;
    let key = DataKey::ReferrerBalance(referrer.clone(), token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    let new_balance = balance.checked_add(reward).ok_or(ErrorCode::Overflow)?;
    e.storage().persistent().set(&key, &new_balance);

    crate::modules::events::emit_referral_reward(e, 0, referrer.clone(), reward);
    Ok(reward)
}

/// Reverse a referral reward that was credited at bet time.
/// Called during cancellation refund to void rewards from cancelled markets.
///
/// Issue #1686/#1687: Takes the exact reward amount that was credited for the
/// bet (as returned by `add_referral_reward`) rather than recomputing it from
/// the fee, so reversal subtracts exactly what was added.
pub fn reverse_referral_reward(e: &Env, referrer: &Address, token: &Address, reward: i128) {
    if reward <= 0 {
        return;
    }
    let key = DataKey::ReferrerBalance(referrer.clone(), token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    let new_balance = balance.saturating_sub(reward);
    e.storage().persistent().set(&key, &new_balance);
}

/// Reverse protocol fee revenue that was collected at bet time.
/// Called during cancellation refund so the fee is returned to the bettor.
/// Issue #1684: returns an error instead of silently saturating when the
/// tracked revenue is insufficient (which would mean fees were already paid
/// out and the refund is under-collateralised).
pub fn reverse_fee(e: &Env, token: Address, amount: i128) -> Result<(), ErrorCode> {
    reverse_fee_for_market(e, 0, token, amount)
}

/// Issue #1684: reverse fees for a specific market.
pub fn reverse_fee_for_market(
    e: &Env,
    market_id: u64,
    token: Address,
    amount: i128,
) -> Result<(), ErrorCode> {
    if amount == 0 {
        return Ok(());
    }

    let key = DataKey::FeeRevenue(token.clone());
    let total: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    let new_total = total.checked_sub(amount).ok_or(ErrorCode::InsufficientBalance)?;
    e.storage().persistent().set(&key, &new_total);

    let market_key = DataKey::MarketFeeRevenue(market_id, token);
    let market_total: i128 = e.storage().persistent().get(&market_key).unwrap_or(0);
    let new_market_total = market_total
        .checked_sub(amount)
        .ok_or(ErrorCode::InsufficientBalance)?;
    e.storage().persistent().set(&market_key, &new_market_total);

    let overall: i128 = e
        .storage()
        .persistent()
        .get(&DataKey::TotalFeesCollected)
        .unwrap_or(0);
    let new_overall = overall
        .checked_sub(amount)
        .ok_or(ErrorCode::InsufficientBalance)?;
    e.storage()
        .persistent()
        .set(&DataKey::TotalFeesCollected, &new_overall);

    Ok(())
}

/// Issue #1: Claim referral rewards for a specific token only.
pub fn claim_referral_rewards(
    e: &Env,
    address: &Address,
    token: &Address,
) -> Result<i128, ErrorCode> {
    // Issue #1685: token egress must respect the emergency pause.
    circuit_breaker::require_not_paused_for_high_risk(e)?;
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

#[cfg(test)]
mod tests {
    use super::{calculate_tiered_fee_with_base, validate_base_fee, MarketTier};
    use crate::errors::ErrorCode;

    #[test]
    fn base_fee_validation_rejects_negative() {
        assert_eq!(validate_base_fee(-1), Err(ErrorCode::FeeTooHigh));
        assert_eq!(validate_base_fee(i128::MIN), Err(ErrorCode::FeeTooHigh));
    }

    #[test]
    fn base_fee_validation_rejects_above_denominator() {
        assert_eq!(validate_base_fee(10_001), Err(ErrorCode::FeeTooHigh));
        assert_eq!(validate_base_fee(i128::MAX), Err(ErrorCode::FeeTooHigh));
    }

    #[test]
    fn base_fee_validation_accepts_in_range() {
        assert_eq!(validate_base_fee(0), Ok(()));
        assert_eq!(validate_base_fee(100), Ok(()));
        assert_eq!(validate_base_fee(10_000), Ok(()));
    }

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
mod collect_fee_events_tests {
    use crate::modules::fees;
    use crate::PredictIQ;
    use soroban_sdk::testutils::{Address as _, Events as _};
    use soroban_sdk::{Address, Env};

    #[test]
    fn collect_fee_emits_fee_collected_event_with_token_address() {
        let env = Env::default();
        let contract_id = env.register(PredictIQ, ());
        let token = Address::generate(&env);

        env.as_contract(&contract_id, || {
            fees::collect_fee(&env, token.clone(), 1_000).unwrap();
        });

        let events = env.events().all();
        assert!(!events.is_empty());

        // Issue: the per-bet fee event must identify which token the fee was
        // collected in, even though it never transfers funds itself.
        let events_debug = format!("{:?}", events);
        assert!(events_debug.contains("fee_colct"));
        let token_debug = format!("{:?}", token);
        assert!(events_debug.contains(&token_debug));
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

    #[test]
    fn test_set_base_fee_rejects_negative_and_oversized() {
        let (env, client, _admin, _token, _contract_id) = setup();

        assert_eq!(
            client.try_set_base_fee(&-1),
            Err(Ok(ErrorCode::FeeTooHigh))
        );
        assert_eq!(
            client.try_set_base_fee(&10_001),
            Err(Ok(ErrorCode::FeeTooHigh))
        );
        // Valid values still accepted
        client.set_base_fee(&10_000);
        assert_eq!(client.get_base_fee(), 10_000);
    }

    #[test]
    fn test_initialize_rejects_invalid_base_fee() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictIQ, ());
        let client = PredictIQClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        assert_eq!(
            client.try_initialize(&admin, &-1),
            Err(Ok(ErrorCode::FeeTooHigh))
        );
        assert_eq!(
            client.try_initialize(&admin, &10_001),
            Err(Ok(ErrorCode::FeeTooHigh))
        );
    }

    #[test]
    fn test_open_market_fees_are_held_back() {
        let (env, client, _admin, token, contract_id) = setup();

        // Fees for an open market (id 0 is treated as open by default) must
        // not be withdrawable.
        seed_fee_revenue(&env, &contract_id, &token, 100_000);

        let treasury = Address::generate(&env);
        let result = client.try_withdraw_protocol_fees(&token, &treasury);
        assert_eq!(result, Err(Ok(ErrorCode::InsufficientBalance)));
        assert_eq!(client.get_revenue(&token), 100_000);
    }

    #[test]
    fn test_reverse_fee_errors_when_revenue_insufficient() {
        let (env, client, _admin, token, contract_id) = setup();

        // No revenue recorded: reversing must fail rather than saturate.
        let result = env.as_contract(&contract_id, || {
            crate::modules::fees::reverse_fee(&env, token.clone(), 1_000)
        });
        assert_eq!(result, Err(ErrorCode::InsufficientBalance));
        let _ = client;
    }

    #[test]
    fn test_contract_balance_covers_outstanding_claims() {
        let (env, client, _admin, token, contract_id) = setup();

        // Collect fees for two markets; only resolved-market fees may leave.
        env.as_contract(&contract_id, || {
            crate::modules::fees::collect_fee_for_market(&env, 1, token.clone(), 40_000)
                .unwrap();
            crate::modules::fees::collect_fee_for_market(&env, 2, token.clone(), 60_000)
                .unwrap();
        });

        let treasury = Address::generate(&env);
        // Market 1 and 2 are open, so nothing is withdrawable yet.
        assert_eq!(
            client.try_withdraw_protocol_fees(&token, &treasury),
            Err(Ok(ErrorCode::InsufficientBalance))
        );

        // Invariant: contract balance >= outstanding (unwithdrawn) claims.
        let contract_balance = token::Client::new(&env, &token).balance(&contract_id);
        let outstanding = client.get_revenue(&token);
        assert!(contract_balance >= outstanding);
    }
}
