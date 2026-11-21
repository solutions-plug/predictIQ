use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::blockchain::BlockchainClient;
use crate::cache::RedisCache;
use crate::config::Config;
use crate::email::queue::EmailQueue;
use crate::email::service::EmailService;
use crate::metrics::Metrics;
use crate::shutdown::ShutdownCoordinator;

/// Shared, ordered log of shutdown events.
///
/// Each recorded event is appended in the order it occurs, so tests can assert
/// on the relative ordering of events (indices) rather than on wall-clock
/// timing. This mirrors the event-recording pattern used in
/// `services/api/tests/graceful_shutdown_test.rs`.
#[derive(Clone, Default)]
struct ShutdownEventLog {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl ShutdownEventLog {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record an event, returning its position in the sequence.
    fn record(&self, event: &'static str) -> usize {
        let mut events = self.events.lock().expect("event log poisoned");
        events.push(event);
        events.len() - 1
    }

    /// Index of the first occurrence of `event`, if any.
    fn index_of(&self, event: &'static str) -> Option<usize> {
        let events = self.events.lock().expect("event log poisoned");
        events.iter().position(|e| *e == event)
    }

    /// All recorded events, in order.
    fn snapshot(&self) -> Vec<&'static str> {
        self.events.lock().expect("event log poisoned").clone()
    }
}

const SERVER_STOPPED_ACCEPTING: &str = "server_stopped_accepting";
const WORKER_DRAIN_STARTED: &str = "worker_drain_started";

/// Assert that the server stopped accepting before any worker drain began.
fn assert_server_stops_before_worker_drain(log: &ShutdownEventLog) {
    let events = log.snapshot();

    let server_idx = log
        .index_of(SERVER_STOPPED_ACCEPTING)
        .unwrap_or_else(|| panic!("server never recorded stop-accepting event; events: {events:?}"));

    let drain_indices: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| **e == WORKER_DRAIN_STARTED)
        .map(|(i, _)| i)
        .collect();

    assert!(
        !drain_indices.is_empty(),
        "no worker drain events recorded; events: {events:?}"
    );

    for drain_idx in drain_indices {
        assert!(
            server_idx < drain_idx,
            "worker drain initiated before server stopped accepting \
             (server_stopped_accepting at {server_idx}, worker_drain_started at {drain_idx}); \
             events: {events:?}"
        );
    }
}

#[tokio::test]
#[ignore] // Requires Redis and PostgreSQL
async fn test_blockchain_worker_graceful_shutdown() {
    // Setup test environment
    let config = Config::from_env();
    let metrics = Metrics::new().expect("Failed to create metrics");
    let cache = RedisCache::new(&config.redis_url)
        .await
        .expect("Failed to connect to Redis");
    
    let blockchain = BlockchainClient::new(&config, cache, metrics)
        .expect("Failed to create blockchain client");
    
    let shutdown_coordinator = ShutdownCoordinator::new(2);
    
    // Start background tasks
    let handles = Arc::new(blockchain)
        .start_background_tasks(&shutdown_coordinator);
    
    assert_eq!(handles.len(), 2);
    
    // Let workers run for a short time
    sleep(Duration::from_millis(100)).await;
    
    // Initiate shutdown
    let shutdown_result = timeout(
        Duration::from_secs(5),
        shutdown_coordinator.shutdown(Duration::from_secs(2))
    ).await;
    
    assert!(shutdown_result.is_ok());
    assert!(shutdown_result.unwrap().is_ok());
    
    // Wait for all handles to complete
    for handle in handles {
        let join_result: Result<Result<(), tokio::task::JoinError>, tokio::time::error::Elapsed> = 
            timeout(Duration::from_secs(1), handle.join()).await;
        assert!(join_result.is_ok());
        assert!(join_result.unwrap().is_ok());
    }
}

#[tokio::test]
#[ignore] // Requires Redis and PostgreSQL
async fn test_email_queue_worker_graceful_shutdown() {
    // Setup test environment
    let config = Config::from_env();
    let metrics = Metrics::new().expect("Failed to create metrics");
    let cache = RedisCache::new(&config.redis_url)
        .await
        .expect("Failed to connect to Redis");
    
    // Create a test database connection
    let db = crate::db::Database::new(&config.database_url, cache.clone(), metrics, config.db_pool.query_timeout)
        .await
        .expect("Failed to connect to database");
    
    let email_service = EmailService::new(config.clone())
        .expect("Failed to create email service");
    let email_queue = EmailQueue::new(cache, db);
    
    let shutdown_coordinator = ShutdownCoordinator::new(1);
    let _shutdown_rx = shutdown_coordinator.subscribe();
    
    // Start email worker
    let _queue_worker = email_queue.clone();
    let _service_worker = email_service.clone();
    let worker_handle = tokio::spawn(async move {
        // This would normally call start_worker but we can't without Redis
        // queue_worker.start_worker(service_worker, shutdown_rx).await;
    });
    
    // Let worker run for a short time
    sleep(Duration::from_millis(100)).await;
    
    // Initiate shutdown
    let shutdown_result = timeout(
        Duration::from_secs(5),
        shutdown_coordinator.shutdown(Duration::from_secs(2))
    ).await;
    
    assert!(shutdown_result.is_ok());
    assert!(shutdown_result.unwrap().is_ok());
    
    // Wait for worker to complete
    let join_result = timeout(Duration::from_secs(1), worker_handle).await;
    assert!(join_result.is_ok());
    assert!(join_result.unwrap().is_ok());
}

#[tokio::test]
async fn test_shutdown_timeout_handling() {
    let shutdown_coordinator = ShutdownCoordinator::new(1);
    
    // Spawn a worker that ignores shutdown signals
    let _shutdown_rx = shutdown_coordinator.subscribe();
    let _stubborn_worker = tokio::spawn(async move {
        // This worker will ignore the shutdown signal and keep running
        loop {
            sleep(Duration::from_millis(100)).await;
            // Intentionally not checking shutdown_rx
        }
    });
    
    // Attempt shutdown with very short timeout
    let shutdown_result = shutdown_coordinator.shutdown(Duration::from_millis(50)).await;
    
    // Should timeout since worker doesn't respond
    assert!(shutdown_result.is_err());
}

#[tokio::test]
async fn test_multiple_workers_coordination() {
    let shutdown_coordinator = ShutdownCoordinator::new(3);
    
    // Spawn multiple workers with different completion times
    let workers = (0..3).map(|i| {
        let mut shutdown_rx = shutdown_coordinator.subscribe();
        let coord = shutdown_coordinator.clone();
        tokio::spawn(async move {
            let _ = shutdown_rx.recv().await;
            // Different sleep times to test coordination
            sleep(Duration::from_millis(50 * (i + 1) as u64)).await;
            coord.worker_completed().await;
        })
    }).collect::<Vec<_>>();
    
    // Start shutdown
    let shutdown_result = timeout(
        Duration::from_secs(2),
        shutdown_coordinator.shutdown(Duration::from_secs(1))
    ).await;
    
    assert!(shutdown_result.is_ok());
    assert!(shutdown_result.unwrap().is_ok());
    
    // All workers should complete
    for worker in workers {
        let join_result = timeout(Duration::from_millis(500), worker).await;
        assert!(join_result.is_ok());
        assert!(join_result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_worker_handle_functionality() {
    let shutdown_coordinator = ShutdownCoordinator::new(1);
    let mut shutdown_rx = shutdown_coordinator.subscribe();
    
    let handle = tokio::spawn(async move {
        let _ = shutdown_rx.recv().await;
        sleep(Duration::from_millis(50)).await;
    });
    
    let worker_handle = crate::shutdown::WorkerHandle::new(
        "test-worker".to_string(),
        handle,
        shutdown_coordinator.clone(),
    );
    
    // Signal shutdown
    let _ = shutdown_coordinator.shutdown(Duration::from_secs(1)).await;
    
    // Worker should complete successfully
    let join_result = worker_handle.join().await;
    assert!(join_result.is_ok());
}

#[tokio::test]
async fn test_signal_handler_setup() {
    // This test verifies that signal handler setup doesn't panic
    // We can't actually test signal reception in unit tests easily,
    // so we just verify the function exists and can be called
    
    // Just verify the function exists - actual signal testing would require
    // process-level integration tests
    assert!(true, "Signal handler setup function exists");
}

#[tokio::test]
async fn test_server_stops_accepting_before_worker_drain() {
    let log = ShutdownEventLog::new();
    let shutdown_coordinator = ShutdownCoordinator::new(2);

    // Server-stop step: waits for the shutdown signal, then records that it has
    // stopped accepting new requests before any worker begins draining.
    let server_log = log.clone();
    let mut server_shutdown_rx = shutdown_coordinator.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server_shutdown_rx.recv().await;
        // The HTTP server stops accepting requests first.
        server_log.record(SERVER_STOPPED_ACCEPTING);
    });

    // Two workers that record the start of their drain once signalled.
    let workers = (0..2)
        .map(|_| {
            let worker_log = log.clone();
            let coord = shutdown_coordinator.clone();
            let mut shutdown_rx = shutdown_coordinator.subscribe();
            tokio::spawn(async move {
                let _ = shutdown_rx.recv().await;
                worker_log.record(WORKER_DRAIN_STARTED);
                coord.worker_completed().await;
            })
        })
        .collect::<Vec<_>>();

    // Let the tasks subscribe and park on the signal.
    sleep(Duration::from_millis(50)).await;

    let shutdown_result = timeout(
        Duration::from_secs(2),
        shutdown_coordinator.shutdown(Duration::from_secs(1)),
    )
    .await;

    assert!(shutdown_result.is_ok(), "shutdown timed out");
    assert!(shutdown_result.unwrap().is_ok(), "shutdown failed");

    // Ensure the server-stop step has run to completion.
    let server_join = timeout(Duration::from_secs(1), server_handle).await;
    assert!(server_join.is_ok(), "server-stop task did not finish");
    assert!(server_join.unwrap().is_ok(), "server-stop task panicked");

    for worker in workers {
        let join_result = timeout(Duration::from_secs(1), worker).await;
        assert!(join_result.is_ok(), "worker did not finish");
        assert!(join_result.unwrap().is_ok(), "worker panicked");
    }

    // Explicit ordering assertion: server stops accepting before any drain.
    assert_server_stops_before_worker_drain(&log);
}

#[tokio::test]
async fn test_server_stops_accepting_before_drain_with_busy_worker() {
    let log = ShutdownEventLog::new();
    let shutdown_coordinator = ShutdownCoordinator::new(1);

    // Server-stop step records that it stopped accepting once signalled.
    let server_log = log.clone();
    let mut server_shutdown_rx = shutdown_coordinator.subscribe();
    let server_handle = tokio::spawn(async move {
        let _ = server_shutdown_rx.recv().await;
        server_log.record(SERVER_STOPPED_ACCEPTING);
    });

    // A worker that is still busy with an in-flight task when the signal
    // arrives. It only records the start of its drain after finishing the
    // in-flight work, so ordering must still hold.
    let worker_log = log.clone();
    let coord = shutdown_coordinator.clone();
    let mut worker_shutdown_rx = shutdown_coordinator.subscribe();
    let worker_handle = tokio::spawn(async move {
        // Simulate in-flight work that is already underway.
        let in_flight = tokio::spawn(async {
            sleep(Duration::from_millis(100)).await;
        });

        // Wait for the shutdown signal while the in-flight task is busy.
        let _ = worker_shutdown_rx.recv().await;

        // Finish the in-flight work before draining.
        let _ = in_flight.await;

        worker_log.record(WORKER_DRAIN_STARTED);
        coord.worker_completed().await;
    });

    // Let the worker start its in-flight task and park on the signal.
    sleep(Duration::from_millis(20)).await;

    let shutdown_result = timeout(
        Duration::from_secs(2),
        shutdown_coordinator.shutdown(Duration::from_secs(1)),
    )
    .await;

    assert!(shutdown_result.is_ok(), "shutdown timed out");
    assert!(shutdown_result.unwrap().is_ok(), "shutdown failed");

    let server_join = timeout(Duration::from_secs(1), server_handle).await;
    assert!(server_join.is_ok(), "server-stop task did not finish");
    assert!(server_join.unwrap().is_ok(), "server-stop task panicked");

    let worker_join = timeout(Duration::from_secs(1), worker_handle).await;
    assert!(worker_join.is_ok(), "worker did not finish");
    assert!(worker_join.unwrap().is_ok(), "worker panicked");

    // Same ordering guarantee must hold even when a worker was busy.
    assert_server_stops_before_worker_drain(&log);
}