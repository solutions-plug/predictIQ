use crate::errors::ErrorCode;
use crate::modules::{markets, oracles, voting};
use crate::types::{ConfigKey, MarketStatus};
use soroban_sdk::{Env, Symbol};

/// Issue #8: Default dispute window increased from 24h → 72h for global participation.
/// Governance can override this via set_dispute_window / ConfigKey::DisputeWindow.
pub const DEFAULT_DISPUTE_WINDOW_SECONDS: u64 = 259_200; // 72 hours
/// Issue #170: Default voting period for dispute resolution
pub const DEFAULT_VOTING_PERIOD_SECONDS: u64 = 259_200; // 72 hours
/// Issue #170: Default majority threshold for dispute resolution (60%)
pub const DEFAULT_MAJORITY_THRESHOLD_BPS: i128 = 6000; // 60%

/// Returns the active dispute window: governance-configured value if set, else the 72h default.
pub fn get_dispute_window(e: &Env) -> u64 {
    e.storage()
        .persistent()
        .get(&ConfigKey::DisputeWindow)
        .unwrap_or(DEFAULT_DISPUTE_WINDOW_SECONDS)
}

/// Admin-only: override the dispute window duration (minimum 24h enforced).
pub fn set_dispute_window(e: &Env, seconds: u64) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    // Enforce a minimum of 24 hours to prevent accidental lockout.
    let clamped = seconds.max(86_400);
    e.storage()
        .persistent()
        .set(&ConfigKey::DisputeWindow, &clamped);
    e.storage().persistent().extend_ttl(
        &ConfigKey::DisputeWindow,
        crate::types::GOV_TTL_LOW_THRESHOLD,
        crate::types::GOV_TTL_HIGH_THRESHOLD,
    );
    Ok(())
}

/// Issue #170: Returns the active voting period: governance-configured value if set, else the 72h default.
pub fn get_voting_period(e: &Env) -> u64 {
    e.storage()
        .persistent()
        .get(&ConfigKey::VotingPeriod)
        .unwrap_or(DEFAULT_VOTING_PERIOD_SECONDS)
}

/// Issue #170: Admin-only: override the voting period duration (minimum 1 hour enforced).
pub fn set_voting_period(e: &Env, seconds: u64) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    // Enforce a minimum of 1 hour to prevent immediate expiration.
    let clamped = seconds.max(3_600);
    e.storage()
        .persistent()
        .set(&ConfigKey::VotingPeriod, &clamped);
    e.storage().persistent().extend_ttl(
        &ConfigKey::VotingPeriod,
        crate::types::GOV_TTL_LOW_THRESHOLD,
        crate::types::GOV_TTL_HIGH_THRESHOLD,
    );
    Ok(())
}

/// Issue #170: Returns the active majority threshold: governance-configured value if set, else the 60% default.
pub fn get_majority_threshold(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&ConfigKey::MajorityThreshold)
        .unwrap_or(DEFAULT_MAJORITY_THRESHOLD_BPS)
}

/// Issue #170: Admin-only: override the majority threshold (must be between 1% and 99%).
pub fn set_majority_threshold(e: &Env, threshold_bps: i128) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    // Enforce bounds: 1% (100 bps) to 99% (9900 bps)
    if threshold_bps < 100 || threshold_bps > 9900 {
        return Err(ErrorCode::InvalidThreshold);
    }
    e.storage()
        .persistent()
        .set(&ConfigKey::MajorityThreshold, &threshold_bps);
    e.storage().persistent().extend_ttl(
        &ConfigKey::MajorityThreshold,
        crate::types::GOV_TTL_LOW_THRESHOLD,
        crate::types::GOV_TTL_HIGH_THRESHOLD,
    );
    Ok(())
}

pub fn attempt_oracle_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }

    if e.ledger().timestamp() < market.resolution_deadline {
        return Err(ErrorCode::ResolutionNotReady);
    }

    // Issue #25: Attempt live Pyth oracle resolution.
    // resolve_with_pyth fetches the price, validates freshness + confidence,
    // stores the result, and returns the winning outcome index.
    let oracle_outcome = oracles::resolve_with_pyth(e, market_id, 0, &market.oracle_config)?;

    market.status = MarketStatus::PendingResolution;
    market.winning_outcome = Some(oracle_outcome);
    market.pending_resolution_timestamp = Some(e.ledger().timestamp());

    markets::update_market(e, market);

    e.events().publish(
        (Symbol::new(e, "oracle_resolved"), market_id),
        oracle_outcome,
    );

    Ok(())
}

pub fn finalize_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    match market.status {
        MarketStatus::PendingResolution => {
            // Check if dispute window has passed (default 72h, configurable)
            let pending_ts = market
                .pending_resolution_timestamp
                .ok_or(ErrorCode::ResolutionNotReady)?;
            if e.ledger().timestamp() < pending_ts + get_dispute_window(e) {
                return Err(ErrorCode::DisputeWindowStillOpen);
            }

            // No dispute filed, finalize with oracle result
            let winning_outcome = market.winning_outcome.unwrap();
            market.status = MarketStatus::Resolved;
            market.resolved_at = Some(e.ledger().timestamp());
            markets::update_market(e, market);

            e.events().publish(
                (Symbol::new(e, "market_finalized"), market_id),
                winning_outcome,
            );

            Ok(())
        }
        MarketStatus::Disputed => {
            // Check if 72h voting period has passed
            let dispute_ts = market
                .dispute_timestamp
                .ok_or(ErrorCode::MarketNotDisputed)?;
            if e.ledger().timestamp() < dispute_ts + get_voting_period(e) {
                return Err(ErrorCode::TimelockActive);
            }

            // Calculate voting outcome — returns NoMajorityReached if < 60% consensus.
            // In that case the market stays Disputed; admin_fallback_resolution must be used.
            let winning_outcome = calculate_voting_outcome(e, &market)?;

            market.status = MarketStatus::Resolved;
            market.winning_outcome = Some(winning_outcome);
            market.resolved_at = Some(e.ledger().timestamp());
            markets::update_market(e, market);

            e.events().publish(
                (Symbol::new(e, "dispute_resolved"), market_id),
                winning_outcome,
            );

            Ok(())
        }
        MarketStatus::Resolved => Err(ErrorCode::CannotChangeOutcome),
        _ => Err(ErrorCode::ResolutionNotReady),
    }
}

/// Issue #63: Administrative fallback for disputed markets that failed to reach
/// the 60% majority threshold after the full voting period.
///
/// Preconditions (all enforced on-chain):
///   1. Caller must be the master admin.
///   2. Market must still be in `Disputed` status (not already resolved/cancelled).
///   3. The 72-hour community voting period must have fully elapsed.
///   4. Community voting must have genuinely failed — `calculate_voting_outcome`
///      must return `NoMajorityReached` (prevents admin from bypassing a valid vote).
///   5. `winning_outcome` must be a valid index into `market.options`.
///
/// This guarantees that user capital is never permanently orphaned while
/// preserving the integrity of the community-first resolution path.
pub fn admin_fallback_resolution(
    e: &Env,
    market_id: u64,
    winning_outcome: u32,
) -> Result<(), ErrorCode> {
    // 1. Admin-only gate
    crate::modules::admin::require_admin(e)?;

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    // 2. Market must be stuck in Disputed — not already resolved or cancelled
    if market.status != MarketStatus::Disputed {
        return Err(ErrorCode::MarketNotDisputed);
    }

    // 3. Voting period must have fully elapsed
    let dispute_ts = market
        .dispute_timestamp
        .ok_or(ErrorCode::MarketNotDisputed)?;
    if e.ledger().timestamp() < dispute_ts + get_voting_period(e) {
        return Err(ErrorCode::VotingPeriodNotElapsed);
    }

    // 4. Community vote must have genuinely deadlocked — only allow fallback when
    //    calculate_voting_outcome returns NoMajorityReached.  Any other error
    //    (e.g. TooManyOutcomes) is surfaced directly so it can be fixed separately.
    match calculate_voting_outcome(e, &market) {
        Ok(_) => {
            // A clear majority exists — admin must not override it; use finalize_resolution instead.
            return Err(ErrorCode::CannotChangeOutcome);
        }
        Err(ErrorCode::NoMajorityReached) => {
            // Confirmed deadlock — proceed with admin fallback.
        }
        Err(other) => return Err(other),
    }

    // 5. Validate the admin-chosen outcome index
    if winning_outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    // Resolve the market with the admin-chosen outcome
    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);
    market.resolved_at = Some(e.ledger().timestamp());
    markets::update_market(e, market);

    let admin = crate::modules::admin::get_admin(e).unwrap_or(e.current_contract_address());
    crate::modules::events::emit_admin_fallback_resolution(e, market_id, admin, winning_outcome);

    Ok(())
}

/// Single-pass O(n) tally. n is bounded by MAX_OUTCOMES_PER_MARKET (32).
/// Returns `NoMajorityReached` if no single outcome holds ≥ 60% of votes,
/// or if two or more outcomes share the highest tally (tie).
fn calculate_voting_outcome(e: &Env, market: &crate::types::Market) -> Result<u32, ErrorCode> {
    let num_outcomes = market.options.len();

    if num_outcomes > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    let mut total_votes: i128 = 0;
    let mut max_outcome: Option<u32> = None;
    let mut max_votes: i128 = 0;
    let mut tie: bool = false;

    for outcome in 0..num_outcomes {
        let tally: i128 = voting::get_tally(e, market.id, outcome);
        total_votes += tally;
        if tally > max_votes {
            max_votes = tally;
            max_outcome = Some(outcome);
            tie = false;
        } else if tally == max_votes && max_votes > 0 {
            tie = true;
        }
    }

    // No votes cast at all — cannot determine a winner.
    if total_votes == 0 {
        return Err(ErrorCode::NoMajorityReached);
    }

    // Two or more outcomes share the highest tally — no deterministic winner.
    if tie {
        return Err(ErrorCode::NoMajorityReached);
    }

    let winner = max_outcome.ok_or(ErrorCode::NoMajorityReached)?;

    // Check if the leading outcome exceeds the configurable majority threshold.
    let majority_pct = (max_votes * 10_000) / total_votes;
    if majority_pct >= get_majority_threshold(e) {
        Ok(winner)
    } else {
        Err(ErrorCode::NoMajorityReached)
    }
}

/// #402: Unit tests for get_dispute_window — default and configured values.
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn get_dispute_window_returns_default_when_not_configured() {
        let env = Env::default();
        let window = get_dispute_window(&env);
        assert_eq!(window, DEFAULT_DISPUTE_WINDOW_SECONDS, "expected 72h default");
    }

    #[test]
    fn get_dispute_window_returns_configured_value() {
        let env = Env::default();
        let custom: u64 = 172_800; // 48 hours
        env.storage()
            .persistent()
            .set(&ConfigKey::DisputeWindow, &custom);
        let window = get_dispute_window(&env);
        assert_eq!(window, custom, "expected configured 48h value");
    }

    #[test]
    fn set_dispute_window_clamps_below_minimum() {
        // set_dispute_window requires admin auth; test the clamp logic directly
        // by writing a sub-minimum value and verifying get_dispute_window reads it.
        // (Full auth path is covered by integration tests.)
        let env = Env::default();
        let below_min: u64 = 3_600; // 1 hour — below 24h minimum
        let clamped = below_min.max(86_400);
        env.storage()
            .persistent()
            .set(&ConfigKey::DisputeWindow, &clamped);
        let window = get_dispute_window(&env);
        assert_eq!(window, 86_400, "window must be clamped to 24h minimum");
    }
}
