use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tokio::time::timeout;
use tracing::{info, warn, error};

/// Shutdown coordinator that manages graceful termination of background workers
#[derive(Clone)]
pub struct ShutdownCoordinator {
    /// Broadcast channel to signal shutdown to all workers
    shutdown_tx: broadcast::Sender<()>,
    /// Watch channel to track worker completion
    completion_tx: watch::Sender<usize>,
    completion_rx: watch::Receiver<usize>,
    /// Total number of workers to wait for
    total_workers: usize,
}

impl ShutdownCoordinator {
    pub fn new(total_workers: usize) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (completion_tx, completion_rx) = watch::channel(0);
        
        Self {
            shutdown_tx,
            completion_tx,
            completion_rx,
            total_workers,
        }
    }

    /// Get a shutdown receiver for a worker
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal that a worker has completed shutdown
    pub async fn worker_completed(&self) {
        let current = *self.completion_rx.borrow();
        let _ = self.completion_tx.send(current + 1);
    }

    /// Initiate graceful shutdown and wait for all workers to complete
    pub async fn shutdown(&self, timeout_duration: Duration) -> anyhow::Result<()> {
        info!("Initiating graceful shutdown for {} workers", self.total_workers);
        
        // Signal all workers to shutdown
        if let Err(e) = self.shutdown_tx.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }

        // Wait for all workers to complete with timeout
        let result = timeout(timeout_duration, self.wait_for_completion()).await;
        
        match result {
            Ok(_) => {
                info!("All workers completed graceful shutdown");
                Ok(())
            }
            Err(_) => {
                error!("Shutdown timeout exceeded, some workers may not have completed");
                Err(anyhow::anyhow!("Shutdown timeout exceeded"))
            }
        }
    }

    async fn wait_for_completion(&self) {
        let mut rx = self.completion_rx.clone();
        
        while *rx.borrow() < self.total_workers {
            if rx.changed().await.is_err() {
                break;
            }
        }
    }
}

/// Handle for a background worker that supports graceful shutdown
pub struct WorkerHandle {
    name: String,
    handle: tokio::task::JoinHandle<()>,
    coordinator: ShutdownCoordinator,
}

impl WorkerHandle {
    pub fn new(
        name: String,
        handle: tokio::task::JoinHandle<()>,
        coordinator: ShutdownCoordinator,
    ) -> Self {
        Self {
            name,
            handle,
            coordinator,
        }
    }

    /// Wait for the worker to complete
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        info!("Waiting for worker '{}' to complete", self.name);
        let result = self.handle.await;
        
        if result.is_ok() {
            info!("Worker '{}' completed successfully", self.name);
        } else {
            error!("Worker '{}' completed with error: {:?}", self.name, result);
        }
        
        self.coordinator.worker_completed().await;
        result
    }

    /// Abort the worker (force termination)
    pub fn abort(&self) {
        warn!("Aborting worker '{}'", self.name);
        self.handle.abort();
    }
}

/// Setup signal handlers for graceful shutdown
pub async fn setup_signal_handlers() -> anyhow::Result<()> {
    use tokio::signal;
    
    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
        
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
            }
        }
    }
    
    #[cfg(windows)]
    {
        let mut ctrl_c = signal::windows::ctrl_c()?;
        let mut ctrl_break = signal::windows::ctrl_break()?;
        let mut ctrl_close = signal::windows::ctrl_close()?;
        let mut ctrl_shutdown = signal::windows::ctrl_shutdown()?;
        
        tokio::select! {
            _ = ctrl_c.recv() => {
                info!("Received Ctrl+C, initiating graceful shutdown");
            }
            _ = ctrl_break.recv() => {
                info!("Received Ctrl+Break, initiating graceful shutdown");
            }
            _ = ctrl_close.recv() => {
                info!("Received Ctrl+Close, initiating graceful shutdown");
            }
            _ = ctrl_shutdown.recv() => {
                info!("Received shutdown signal, initiating graceful shutdown");
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_shutdown_coordinator() {
        let coordinator = ShutdownCoordinator::new(2);
        
        // Spawn two mock workers
        let coord1 = coordinator.clone();
        let coord2 = coordinator.clone();
        
        let worker1 = tokio::spawn(async move {
            let mut shutdown_rx = coord1.subscribe();
            let _ = shutdown_rx.recv().await;
            sleep(Duration::from_millis(100)).await;
            coord1.worker_completed().await;
        });
        
        let worker2 = tokio::spawn(async move {
            let mut shutdown_rx = coord2.subscribe();
            let _ = shutdown_rx.recv().await;
            sleep(Duration::from_millis(50)).await;
            coord2.worker_completed().await;
        });
        
        // Start shutdown
        let shutdown_result = coordinator.shutdown(Duration::from_secs(1)).await;
        
        // Wait for workers
        let _ = worker1.await;
        let _ = worker2.await;
        
        assert!(shutdown_result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        let coordinator = ShutdownCoordinator::new(1);
        
        // Spawn a worker that takes too long
        let coord = coordinator.clone();
        let _worker = tokio::spawn(async move {
            let mut shutdown_rx = coord.subscribe();
            let _ = shutdown_rx.recv().await;
            sleep(Duration::from_secs(2)).await; // Takes longer than timeout
            coord.worker_completed().await;
        });
        
        // Start shutdown with short timeout
        let shutdown_result = coordinator.shutdown(Duration::from_millis(100)).await;
        
        assert!(shutdown_result.is_err());
    }
}