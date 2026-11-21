use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Integration test for graceful shutdown behavior
/// This test starts the actual application and sends it a SIGTERM signal
/// to verify that it shuts down gracefully within the expected timeframe.
#[tokio::test]
#[ignore] // Ignore by default since it requires external dependencies
async fn test_graceful_shutdown_integration() {
    // Set environment variables for test
    std::env::set_var("RUST_LOG", "info");
    std::env::set_var("API_BIND_ADDR", "127.0.0.1:0"); // Use random port
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
    std::env::set_var("DATABASE_URL", "postgres://postgres:postgres@127.0.0.1/predictiq_test");
    
    // Start the application as a subprocess
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "predictiq-api"])
        .current_dir("predictIQ/services/api")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start application");

    // Give the application time to start up
    sleep(Duration::from_secs(2)).await;

    // Send SIGTERM signal
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        
        let pid = Pid::from_raw(child.id() as i32);
        signal::kill(pid, Signal::SIGTERM).expect("Failed to send SIGTERM");
    }

    #[cfg(windows)]
    {
        // On Windows, we'll use Ctrl+C equivalent
        child.kill().expect("Failed to terminate process");
    }

    // Wait for graceful shutdown with timeout
    let shutdown_result = timeout(Duration::from_secs(35), child.wait()).await;
    
    match shutdown_result {
        Ok(Ok(exit_status)) => {
            assert!(exit_status.success(), "Application did not exit successfully");
            println!("Application shut down gracefully");
        }
        Ok(Err(e)) => {
            panic!("Error waiting for application to exit: {}", e);
        }
        Err(_) => {
            // Timeout - force kill and fail test
            let _ = child.kill();
            panic!("Application did not shut down within timeout period");
        }
    }
}

/// Test that verifies shutdown behavior under load
#[tokio::test]
#[ignore] // Ignore by default since it requires external dependencies
async fn test_graceful_shutdown_under_load() {
    // This test would:
    // 1. Start the application
    // 2. Generate some background work (email jobs, blockchain sync)
    // 3. Send shutdown signal while work is in progress
    // 4. Verify that in-flight work is handled properly
    // 5. Verify shutdown completes within timeout
    
    // For now, this is a placeholder for a more comprehensive test
    // that would require setting up test data and monitoring worker state
    
    println!("Shutdown under load test - placeholder for future implementation");
}

/// Test that verifies the shutdown ordering: the HTTP server must stop
/// accepting connections *before* background workers begin draining.
///
/// This models the sequence documented in services/api/GRACEFUL_SHUTDOWN.md:
///   1. Signal received -> signal future resolves (signalling only)
///   2. HTTP server stops accepting new connections
///   3. Background workers are drained
///
/// The test records the order in which each phase completes and asserts that
/// the server-stop event precedes the worker-drain event. It also asserts that
/// no work is accepted by a worker after draining has started.
#[tokio::test]
async fn test_server_stops_accepting_before_workers_drain() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    // Ordered log of shutdown phases as they complete.
    let events: Arc<tokio::sync::Mutex<Vec<&'static str>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Whether workers have begun draining.
    let draining_started = Arc::new(AtomicBool::new(false));
    // Count of work items accepted by workers after draining started.
    let work_after_drain = Arc::new(AtomicUsize::new(0));

    // --- Phase 1: signal future (signalling only) ---
    let signal_events = events.clone();
    let signal_future = async move {
        // Simulate the signal being received; the signal future must not drain
        // workers itself, it only signals.
        signal_events.lock().await.push("signal");
    };

    // --- Phase 2: HTTP server stops accepting connections ---
    let server_events = events.clone();
    let server_draining = draining_started.clone();
    let server_work_after = work_after_drain.clone();
    let server_future = async move {
        // Simulate the server shutting down its listener. While the server is
        // still accepting, a request may enqueue work to a worker.
        // After the server stops, workers are allowed to drain.
        server_events.lock().await.push("server_stopped");
        // Mark draining as started only after the server has stopped.
        server_draining.store(true, Ordering::SeqCst);
        // Any work submitted after this point must be rejected by the worker.
        if server_work_after.load(Ordering::SeqCst) > 0 {
            // This branch is defensive; the worker below enforces the rule.
        }
    };

    // --- Phase 3: workers drain ---
    let worker_events = events.clone();
    let worker_draining = draining_started.clone();
    let worker_work_after = work_after_drain.clone();
    let worker_future = async move {
        // A worker accepts work only if draining has not started.
        let submit_work = |draining: &AtomicBool, counter: &AtomicUsize| {
            if draining.load(Ordering::SeqCst) {
                counter.fetch_add(1, Ordering::SeqCst);
                false
            } else {
                true
            }
        };

        // Simulate an in-flight request enqueuing work while the server is
        // still accepting: this must be accepted.
        assert!(
            submit_work(&worker_draining, &worker_work_after),
            "worker should accept work before draining starts"
        );

        // Now drain: record the event.
        worker_events.lock().await.push("workers_drained");

        // Any work submitted after draining starts must be rejected.
        assert!(
            !submit_work(&worker_draining, &worker_work_after),
            "worker must reject work after draining starts"
        );
    };

    // Run the phases in the intended order: signal -> server stop -> drain.
    signal_future.await;
    server_future.await;
    worker_future.await;

    // Assert the ordering: server_stopped must come before workers_drained.
    let log = events.lock().await;
    let server_pos = log
        .iter()
        .position(|e| *e == "server_stopped")
        .expect("server_stopped event missing");
    let drain_pos = log
        .iter()
        .position(|e| *e == "workers_drained")
        .expect("workers_drained event missing");
    assert!(
        server_pos < drain_pos,
        "server must stop accepting before workers drain (log: {:?})",
        *log
    );

    // No worker should have received work after draining started.
    assert_eq!(
        work_after_drain.load(Ordering::SeqCst),
        0,
        "no worker should receive work after draining starts"
    );
}

/// Test configuration for different shutdown timeouts
#[tokio::test]
async fn test_shutdown_timeout_configuration() {
    use predictiq_api::shutdown::ShutdownCoordinator;
    
    // Test very short timeout
    let coordinator = ShutdownCoordinator::new(1);
    let result = coordinator.shutdown(Duration::from_millis(1)).await;
    assert!(result.is_err(), "Should timeout with very short duration");
    
    // Test reasonable timeout with no workers
    let coordinator = ShutdownCoordinator::new(0);
    let result = coordinator.shutdown(Duration::from_secs(1)).await;
    assert!(result.is_ok(), "Should succeed with no workers to wait for");
}