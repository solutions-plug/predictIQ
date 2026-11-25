pub mod admin;
pub mod bets;
pub mod cancellation;
pub mod circuit_breaker;
pub mod disputes;
pub mod event_archive;
pub mod events;
pub mod fees;
pub mod governance;
pub mod markets;
pub mod migration;
pub mod monitoring;
pub mod oracles;
pub mod queries;
pub mod resolution;
pub mod sac;
pub mod voting;

#[cfg(test)]
mod admin_test;
#[cfg(test)]
mod bets_test;
#[cfg(test)]
mod bets_fuzz_test;
#[cfg(test)]
mod circuit_breaker_test;
#[cfg(test)]
mod disputes_weight_test;
#[cfg(test)]
mod event_archive_storage_test;
#[cfg(test)]
mod events_test;
#[cfg(test)]
mod governance_guardian_test;
#[cfg(test)]
mod invariants_test;
#[cfg(test)]
mod markets_conditional_test;
#[cfg(test)]
mod migration_history_test;
#[cfg(test)]
mod oracles_test;
#[cfg(test)]
mod property_invariants_test;