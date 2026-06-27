use std::time::Duration;
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Read `EMAIL_QUEUE_DRAIN_TIMEOUT_SECS` from the environment.
/// Defaults to 60 s — more generous than the global shutdown timeout
/// because losing in-flight emails is more expensive than delaying exit.
pub fn email_queue_drain_timeout() -> Duration {
    let secs = std::env::var("EMAIL_QUEUE_DRAIN_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    Duration::from_secs(secs)
}

/// Coordinates graceful shutdown across all background workers.
///
/// Workers receive a [`CancellationToken`] they poll on each iteration.
/// When [`ShutdownCoordinator::shutdown`] is called the token is cancelled,
/// workers finish their current in-flight task, then decrement the completion
/// counter via [`ShutdownCoordinator::worker_completed`].
#[derive(Clone)]
pub struct ShutdownCoordinator {
    /// Token cancelled when shutdown is initiated.
    token: CancellationToken,
    /// Tracks how many workers have finished.
    completion_tx: watch::Sender<usize>,
    completion_rx: watch::Receiver<usize>,
    total_workers: usize,
}

impl ShutdownCoordinator {
    pub fn new(total_workers: usize) -> Self {
        let (completion_tx, completion_rx) = watch::channel(0);
        Self {
            token: CancellationToken::new(),
            completion_tx,
            completion_rx,
            total_workers,
        }
    }

    /// Returns a child token that workers should poll with `.is_cancelled()`.
    pub fn token(&self) -> CancellationToken {
        self.token.child_token()
    }

    /// Subscribe to the raw broadcast — kept for backward-compat with tests.
    pub fn subscribe(&self) -> CancellationToken {
        self.token.child_token()
    }

    /// Called by each worker once it has finished its last in-flight task.
    pub fn worker_completed(&self) {
        let current = *self.completion_rx.borrow();
        let _ = self.completion_tx.send(current + 1);
    }

    /// Cancel the token and wait up to `timeout_duration` for all workers.
    /// Returns `Err` if the timeout is exceeded (callers should force-exit).
    pub async fn shutdown(&self, timeout_duration: Duration) -> anyhow::Result<()> {
        info!(
            total_workers = self.total_workers,
            "Initiating graceful shutdown"
        );
        self.token.cancel();

        match timeout(timeout_duration, self.wait_for_completion()).await {
            Ok(_) => {
                info!("All workers completed graceful shutdown");
                Ok(())
            }
            Err(_) => {
                let done = *self.completion_rx.borrow();
                error!(
                    completed = done,
                    total = self.total_workers,
                    "Shutdown timeout exceeded — forcing exit"
                );
                Err(anyhow::anyhow!(
                    "Shutdown timeout: {}/{} workers completed",
                    done,
                    self.total_workers
                ))
            }
        }
    }

    async fn wait_for_completion(&self) {
        let mut rx = self.completion_rx.clone();
        loop {
            if *rx.borrow() >= self.total_workers {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    }
}

/// A handle to a spawned background worker.
pub struct WorkerHandle {
    name: String,
    handle: tokio::task::JoinHandle<()>,
}

impl WorkerHandle {
    pub fn new(name: impl Into<String>, handle: tokio::task::JoinHandle<()>) -> Self {
        Self {
            name: name.into(),
            handle,
        }
    }

    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        info!(worker = %self.name, "Waiting for worker to finish");
        let result = self.handle.await;
        match &result {
            Ok(_) => info!(worker = %self.name, "Worker finished cleanly"),
            Err(e) => error!(worker = %self.name, error = %e, "Worker panicked"),
        }
        result
    }

    pub fn abort(&self) {
        warn!(worker = %self.name, "Aborting worker (force)");
        self.handle.abort();
    }
}

/// Waits for SIGTERM or SIGINT (cross-platform).
pub async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv()  => info!("Received SIGINT"),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        info!("Received Ctrl+C");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_workers_stop_on_cancellation() {
        let coord = ShutdownCoordinator::new(2);

        for _ in 0..2 {
            let token = coord.token();
            let coord2 = coord.clone();
            tokio::spawn(async move {
                token.cancelled().await;
                sleep(Duration::from_millis(20)).await;
                coord2.worker_completed();
            });
        }

        let result = coord.shutdown(Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_timeout_forces_exit() {
        let coord = ShutdownCoordinator::new(1);

        // Worker that never calls worker_completed
        let token = coord.token();
        tokio::spawn(async move {
            token.cancelled().await;
            sleep(Duration::from_secs(60)).await; // far longer than timeout
        });

        let result = coord.shutdown(Duration::from_millis(80)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_inflight_work_completes_before_shutdown() {
        let coord = ShutdownCoordinator::new(1);
        let work_done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let work_done2 = work_done.clone();

        let token = coord.token();
        let coord2 = coord.clone();
        tokio::spawn(async move {
            token.cancelled().await;
            // Simulate finishing in-flight work
            sleep(Duration::from_millis(50)).await;
            work_done2.store(true, std::sync::atomic::Ordering::SeqCst);
            coord2.worker_completed();
        });

        coord.shutdown(Duration::from_secs(1)).await.unwrap();
        assert!(work_done.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_no_new_work_after_signal() {
        // Verify that once the token is cancelled, workers stop dequeuing.
        let coord = ShutdownCoordinator::new(1);
        let iterations = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let iterations2 = iterations.clone();

        let token = coord.token();
        let coord2 = coord.clone();
        tokio::spawn(async move {
            loop {
                if token.is_cancelled() {
                    break;
                }
                iterations2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                sleep(Duration::from_millis(10)).await;
            }
            coord2.worker_completed();
        });

        sleep(Duration::from_millis(35)).await;
        let before = iterations.load(std::sync::atomic::Ordering::SeqCst);
        coord.shutdown(Duration::from_secs(1)).await.unwrap();
        let after = iterations.load(std::sync::atomic::Ordering::SeqCst);

        // No new iterations after shutdown
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn test_multiple_workers_all_complete() {
        let coord = ShutdownCoordinator::new(3);

        for i in 0..3u64 {
            let token = coord.token();
            let coord2 = coord.clone();
            tokio::spawn(async move {
                token.cancelled().await;
                sleep(Duration::from_millis(10 * (i + 1))).await;
                coord2.worker_completed();
            });
        }

        let result = coord.shutdown(Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }
}
