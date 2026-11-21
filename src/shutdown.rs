//! Graceful shutdown coordination and worker supervision.
//!
//! This module provides the [`ShutdownCoordinator`] used to signal background
//! tasks to stop, as well as the [`WorkerSupervisor`] which encapsulates the
//! monitored restart loop used for the email queue worker (exponential backoff,
//! restart cap, FATAL logging and crash metrics).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;

/// Maximum number of times the email queue worker will be restarted before the
/// supervisor gives up and logs a `FATAL` message.
pub const MAX_EMAIL_WORKER_RESTARTS: u32 = 5;

/// Base delay used for the exponential backoff between worker restarts.
pub const EMAIL_WORKER_BACKOFF_BASE: Duration = Duration::from_millis(500);

/// Coordinates graceful shutdown across background tasks.
///
/// Shutdown proceeds in two distinct phases so that in-flight requests are
/// never able to enqueue work onto workers that have already drained:
///
/// 1. The signal-handling future only *signals* shutdown (via [`Self::shutdown`])
///    and returns. It never drains workers itself.
/// 2. The main task observes the signal, stops the HTTP server from accepting
///    new connections, and only then drains background workers.
///
/// See `services/api/GRACEFUL_SHUTDOWN.md` for the documented sequence.
#[derive(Clone)]
pub struct ShutdownCoordinator {
    tx: watch::Sender<bool>,
    rx: watch::Receiver<bool>,
}

impl ShutdownCoordinator {
    /// Create a new coordinator with no shutdown requested.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self { tx, rx }
    }

    /// Request shutdown of all subscribed tasks.
    ///
    /// This only flips the signal; it does not drain workers. Draining is the
    /// responsibility of the main task, which must first stop the HTTP server
    /// from accepting connections.
    pub fn shutdown(&self) {
        let _ = self.tx.send(true);
    }

    /// Returns `true` if shutdown has been requested.
    pub fn is_shutdown(&self) -> bool {
        *self.rx.borrow()
    }

    /// Wait until shutdown is requested.
    pub async fn wait(&mut self) {
        if self.is_shutdown() {
            return;
        }
        let _ = self.rx.changed().await;
    }

    /// Signal-only future suitable for `tokio::select!` against the server.
    ///
    /// Resolves once shutdown has been requested and performs no draining, so
    /// the main task can stop accepting connections before draining workers.
    pub async fn signalled(&mut self) {
        self.wait().await;
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the exponential backoff duration for a given restart attempt.
///
/// Attempt `0` yields the base delay, and each subsequent attempt doubles it.
pub fn email_worker_backoff(attempt: u32) -> Duration {
    EMAIL_WORKER_BACKOFF_BASE * 2u32.saturating_pow(attempt)
}

/// Outcome of a supervised worker run.
#[derive(Debug, PartialEq, Eq)]
pub enum SupervisorOutcome {
    /// The worker exited cleanly (no restart needed).
    Completed,
    /// The worker crashed and was restarted `attempts` times before succeeding.
    Recovered { attempts: u32 },
    /// The worker exceeded [`MAX_EMAIL_WORKER_RESTARTS`] and the supervisor gave up.
    Exhausted { attempts: u32 },
}

/// Supervises a fallible worker, restarting it with exponential backoff up to
/// [`MAX_EMAIL_WORKER_RESTARTS`] times and recording crash metrics.
///
/// The supervisor is generic over the worker closure so the restart-loop logic
/// can be unit tested without spawning the real email queue worker.
pub struct WorkerSupervisor {
    max_restarts: u32,
    worker_crash_total: Arc<AtomicU64>,
}

impl WorkerSupervisor {
    /// Create a supervisor with the default restart cap.
    pub fn new(worker_crash_total: Arc<AtomicU64>) -> Self {
        Self {
            max_restarts: MAX_EMAIL_WORKER_RESTARTS,
            worker_crash_total,
        }
    }

    /// Create a supervisor with an explicit restart cap (used in tests).
    pub fn with_max_restarts(max_restarts: u32, worker_crash_total: Arc<AtomicU64>) -> Self {
        Self {
            max_restarts,
            worker_crash_total,
        }
    }

    /// Number of crashes recorded so far.
    pub fn crash_total(&self) -> u64 {
        self.worker_crash_total.load(Ordering::SeqCst)
    }

    /// Run `worker`, restarting it on failure with exponential backoff.
    ///
    /// Each crash increments `worker_crash_total` exactly once. After
    /// `max_restarts` crashes the supervisor logs `FATAL` and returns
    /// [`SupervisorOutcome::Exhausted`].
    pub async fn supervise<F, Fut, E>(&self, mut worker: F) -> SupervisorOutcome
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<(), E>>,
    {
        let mut attempts: u32 = 0;
        loop {
            match worker().await {
                Ok(()) => {
                    return if attempts == 0 {
                        SupervisorOutcome::Completed
                    } else {
                        SupervisorOutcome::Recovered { attempts }
                    };
                }
                Err(_) => {
                    self.worker_crash_total.fetch_add(1, Ordering::SeqCst);
                    attempts += 1;

                    if attempts > self.max_restarts {
                        tracing::error!(
                            attempts,
                            max_restarts = self.max_restarts,
                            "FATAL: email queue worker exceeded MAX_EMAIL_WORKER_RESTARTS; giving up"
                        );
                        return SupervisorOutcome::Exhausted { attempts };
                    }

                    let backoff = email_worker_backoff(attempts - 1);
                    tracing::warn!(
                        attempt = attempts,
                        backoff_ms = backoff.as_millis() as u64,
                        "email queue worker crashed; restarting after backoff"
                    );
                    sleep(backoff).await;
                }
            }
        }
    }
}

/// Ordered shutdown phases, used to assert that the HTTP server stops
/// accepting connections before background workers begin draining.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    /// The process is running normally.
    Running,
    /// Shutdown has been signalled; the server is no longer accepting.
    ServerStopped,
    /// Background workers are being drained.
    WorkersDraining,
    /// Shutdown is complete.
    Complete,
}

/// Drives the ordered graceful shutdown sequence.
///
/// The signal future only calls [`ShutdownCoordinator::shutdown`]; this helper
/// runs in the main task and enforces the ordering:
/// stop accepting connections, then drain workers.
pub struct GracefulShutdown {
    coordinator: ShutdownCoordinator,
    phase: ShutdownPhase,
}

impl GracefulShutdown {
    /// Create a new shutdown driver around `coordinator`.
    pub fn new(coordinator: ShutdownCoordinator) -> Self {
        Self {
            coordinator,
            phase: ShutdownPhase::Running,
        }
    }

    /// Current shutdown phase.
    pub fn phase(&self) -> ShutdownPhase {
        self.phase
    }

    /// Signal-only future: requests shutdown and returns without draining.
    pub async fn signal_future(&mut self) {
        self.coordinator.signalled().await;
    }

    /// Run the ordered shutdown sequence in the main task.
    ///
    /// `stop_accepting` is awaited first (the HTTP server stops accepting new
    /// connections), and only afterwards is `drain_workers` awaited.
    pub async fn run<StopAccepting, StopFut, Drain, DrainFut>(
        &mut self,
        stop_accepting: StopAccepting,
        drain_workers: Drain,
    ) where
        StopAccepting: FnOnce() -> StopFut,
        StopFut: std::future::Future<Output = ()>,
        Drain: FnOnce() -> DrainFut,
        DrainFut: std::future::Future<Output = ()>,
    {
        stop_accepting().await;
        self.phase = ShutdownPhase::ServerStopped;

        drain_workers().await;
        self.phase = ShutdownPhase::WorkersDraining;

        self.phase = ShutdownPhase::Complete;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn backoff_sequence_doubles_each_attempt() {
        assert_eq!(email_worker_backoff(0), Duration::from_millis(500));
        assert_eq!(email_worker_backoff(1), Duration::from_millis(1000));
        assert_eq!(email_worker_backoff(2), Duration::from_millis(2000));
        assert_eq!(email_worker_backoff(3), Duration::from_millis(4000));
        assert_eq!(email_worker_backoff(4), Duration::from_millis(8000));
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_restarts_exactly_max_times_then_exhausts() {
        let crashes = Arc::new(AtomicU64::new(0));
        let supervisor = WorkerSupervisor::new(crashes.clone());

        let outcome = supervisor
            .supervise(|| async { Err::<(), ()>(()) })
            .await;

        assert_eq!(
            outcome,
            SupervisorOutcome::Exhausted {
                attempts: MAX_EMAIL_WORKER_RESTARTS + 1
            }
        );
        // One crash per failed attempt, including the final one that trips the cap.
        assert_eq!(supervisor.crash_total(), (MAX_EMAIL_WORKER_RESTARTS + 1) as u64);
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_records_one_crash_per_failure() {
        let crashes = Arc::new(AtomicU64::new(0));
        let supervisor = WorkerSupervisor::with_max_restarts(3, crashes.clone());

        let mut calls = 0u32;
        let outcome = supervisor
            .supervise(|| {
                calls += 1;
                let current = calls;
                async move {
                    if current < 3 {
                        Err::<(), ()>(())
                    } else {
                        Ok(())
                    }
                }
            })
            .await;

        assert_eq!(outcome, SupervisorOutcome::Recovered { attempts: 2 });
        assert_eq!(supervisor.crash_total(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_completes_without_crash() {
        let crashes = Arc::new(AtomicU64::new(0));
        let supervisor = WorkerSupervisor::new(crashes.clone());

        let outcome = supervisor.supervise(|| async { Ok::<(), ()>(()) }).await;

        assert_eq!(outcome, SupervisorOutcome::Completed);
        assert_eq!(supervisor.crash_total(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn server_stops_accepting_before_workers_drain() {
        let coordinator = ShutdownCoordinator::new();
        let mut shutdown = GracefulShutdown::new(coordinator.clone());

        // The signal future only signals; it must not drain workers.
        let signal = tokio::spawn(async move {
            coordinator.shutdown();
        });
        shutdown.signal_future().await;
        signal.await.unwrap();

        assert_eq!(shutdown.phase(), ShutdownPhase::Running);

        let events = Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));

        let stop_events = events.clone();
        let drain_events = events.clone();
        shutdown
            .run(
                move || async move {
                    stop_events.lock().unwrap().push("stop_accepting");
                },
                move || async move {
                    drain_events.lock().unwrap().push("drain_workers");
                },
            )
            .await;

        assert_eq!(
            *events.lock().unwrap(),
            vec!["stop_accepting", "drain_workers"]
        );
        assert_eq!(shutdown.phase(), ShutdownPhase::Complete);
    }

    #[tokio::test(start_paused = true)]
    async fn no_work_dispatched_after_draining_starts() {
        let coordinator = ShutdownCoordinator::new();
        let mut shutdown = GracefulShutdown::new(coordinator.clone());

        let accepting = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let dispatched_after_drain = Arc::new(AtomicU64::new(0));

        let accepting_for_stop = accepting.clone();
        let accepting_for_drain = accepting.clone();
        let dispatched = dispatched_after_drain.clone();

        shutdown
            .run(
                move || async move {
                    accepting_for_stop.store(false, Ordering::SeqCst);
                },
                move || async move {
                    // Simulate a worker draining; any dispatch attempt while
                    // the server is no longer accepting must be rejected.
                    if !accepting_for_drain.load(Ordering::SeqCst) {
                        // Server already stopped accepting: no new work.
                    } else {
                        dispatched.fetch_add(1, Ordering::SeqCst);
                    }
                },
            )
            .await;

        assert!(!accepting.load(Ordering::SeqCst));
        assert_eq!(dispatched_after_drain.load(Ordering::SeqCst), 0);
    }
}
