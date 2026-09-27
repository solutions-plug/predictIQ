//! Supervised restart loop for the email queue worker.
//!
//! This module extracts the crash-loop handling that previously lived inline in
//! `main.rs` so the backoff sequence, restart cap, and crash metric can be unit
//! tested without spawning the real worker.

use std::time::Duration;

/// Maximum number of restarts before the supervisor gives up and logs `FATAL`.
pub const MAX_EMAIL_WORKER_RESTARTS: u32 = 5;

/// Base delay used for the exponential backoff between restarts.
pub const EMAIL_WORKER_BACKOFF_BASE: Duration = Duration::from_millis(500);

/// Outcome of a single supervised worker run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerOutcome {
    /// The worker exited cleanly and should not be restarted.
    Completed,
    /// The worker crashed (e.g. panicked) and may be restarted.
    Crashed,
}

/// Computes the exponential backoff delay for a given (zero-based) restart attempt.
///
/// Attempt 0 -> base, attempt 1 -> 2x base, attempt 2 -> 4x base, ...
pub fn backoff_for_attempt(attempt: u32) -> Duration {
    let multiplier = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
    EMAIL_WORKER_BACKOFF_BASE.saturating_mul(multiplier)
}

/// Tracks the state of the email worker restart loop.
///
/// The supervisor records how many times the worker has crashed and how many
/// restarts have been attempted, exposing the backoff delay for the next
/// restart and the crash counter that feeds the `worker_crash_total` metric.
#[derive(Debug, Default)]
pub struct WorkerSupervisor {
    restarts: u32,
    crashes: u64,
}

impl WorkerSupervisor {
    /// Creates a fresh supervisor with no recorded restarts or crashes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of restart attempts made so far.
    pub fn restarts(&self) -> u32 {
        self.restarts
    }

    /// Value of the `worker_crash_total` metric (one increment per crash).
    pub fn crash_total(&self) -> u64 {
        self.crashes
    }

    /// Records a crash and returns the backoff delay to wait before the next
    /// restart, or `None` when the restart cap has been reached.
    ///
    /// Each call increments the crash counter exactly once. When the cap is
    /// exceeded the caller is expected to log `FATAL` and stop restarting.
    pub fn record_crash(&mut self) -> Option<Duration> {
        self.crashes += 1;

        if self.restarts >= MAX_EMAIL_WORKER_RESTARTS {
            return None;
        }

        let delay = backoff_for_attempt(self.restarts);
        self.restarts += 1;
        Some(delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_restarts_at_max_email_worker_restarts() {
        let mut supervisor = WorkerSupervisor::new();

        for _ in 0..MAX_EMAIL_WORKER_RESTARTS {
            assert!(supervisor.record_crash().is_some());
        }

        assert_eq!(supervisor.restarts(), MAX_EMAIL_WORKER_RESTARTS);

        // The next crash exceeds the cap and must not schedule another restart.
        assert_eq!(supervisor.record_crash(), None);
        assert_eq!(supervisor.restarts(), MAX_EMAIL_WORKER_RESTARTS);
    }

    #[test]
    fn backoff_doubles_for_each_restart_attempt() {
        let mut supervisor = WorkerSupervisor::new();
        let mut delays = Vec::new();

        for _ in 0..MAX_EMAIL_WORKER_RESTARTS {
            delays.push(supervisor.record_crash().expect("restart within cap"));
        }

        let expected: Vec<Duration> = (0..MAX_EMAIL_WORKER_RESTARTS)
            .map(backoff_for_attempt)
            .collect();

        assert_eq!(delays, expected);
        assert_eq!(delays[0], EMAIL_WORKER_BACKOFF_BASE);
        assert_eq!(delays[1], EMAIL_WORKER_BACKOFF_BASE * 2);
        assert_eq!(delays[2], EMAIL_WORKER_BACKOFF_BASE * 4);
        assert_eq!(delays[3], EMAIL_WORKER_BACKOFF_BASE * 8);
        assert_eq!(delays[4], EMAIL_WORKER_BACKOFF_BASE * 16);
    }

    #[test]
    fn crash_total_increments_once_per_crash() {
        let mut supervisor = WorkerSupervisor::new();

        for expected in 1..=MAX_EMAIL_WORKER_RESTARTS as u64 {
            supervisor.record_crash();
            assert_eq!(supervisor.crash_total(), expected);
        }

        // A crash past the cap still increments the metric exactly once.
        supervisor.record_crash();
        assert_eq!(supervisor.crash_total(), MAX_EMAIL_WORKER_RESTARTS as u64 + 1);
    }
}
