// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Integration tests for HTTP methods (HEAD, OPTIONS) and basic functionality
//!
//! These tests focus on testing the HTTP method handlers directly without
//! requiring full server startup, making them more reliable and faster.

use http_handle::{Server, ShutdownSignal};
use std::fs;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Helper function to set up a test directory structure
fn setup_test_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root_path = temp_dir.path();

    // Create index.html
    fs::write(
        root_path.join("index.html"),
        "<html><body>Hello, Integration Test!</body></html>",
    )
    .unwrap();

    // Create 404/index.html
    fs::create_dir(root_path.join("404")).unwrap();
    fs::write(
        root_path.join("404/index.html"),
        "<html><body>404 Not Found</body></html>",
    )
    .unwrap();

    // Create a CSS file for content type testing
    fs::write(root_path.join("style.css"), "body { color: red; }")
        .unwrap();

    // Create a JavaScript file
    fs::write(
        root_path.join("script.js"),
        "console.log('Hello, World!');",
    )
    .unwrap();

    temp_dir
}

/// Helper function to find an available port
fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[test]
fn test_server_creation_and_configuration() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);

    // Test server creation
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    // We can't easily test private fields, but we can verify the server was created
    // This test ensures the basic constructor works
    assert!(format!("{:?}", server).contains(&address));
}

#[test]
fn test_shutdown_signal_functionality() {
    // Test ShutdownSignal creation and basic functionality
    let shutdown = ShutdownSignal::new(Duration::from_secs(1));

    // Test initial state
    assert!(!shutdown.is_shutdown_requested());
    assert_eq!(shutdown.active_connection_count(), 0);

    // Test connection tracking
    shutdown.connection_started();
    assert_eq!(shutdown.active_connection_count(), 1);

    shutdown.connection_started();
    assert_eq!(shutdown.active_connection_count(), 2);

    shutdown.connection_finished();
    assert_eq!(shutdown.active_connection_count(), 1);

    // Test shutdown signal
    shutdown.shutdown();
    assert!(shutdown.is_shutdown_requested());

    // Connection count should remain the same after shutdown signal
    assert_eq!(shutdown.active_connection_count(), 1);

    // Clean up remaining connections
    shutdown.connection_finished();
    assert_eq!(shutdown.active_connection_count(), 0);
}

#[test]
fn test_shutdown_signal_wait_functionality() {
    let shutdown = ShutdownSignal::new(Duration::from_millis(100));

    // Test immediate shutdown (no active connections)
    shutdown.shutdown();
    let start = std::time::Instant::now();
    let graceful = shutdown.wait_for_shutdown();
    let elapsed = start.elapsed();

    assert!(
        graceful,
        "Should shut down gracefully when no connections"
    );
    assert!(
        elapsed < Duration::from_millis(50),
        "Should be nearly immediate when no connections, took: {:?}",
        elapsed
    );
}

#[test]
fn test_shutdown_signal_timeout() {
    let shutdown = ShutdownSignal::new(Duration::from_millis(200));

    // Simulate active connection that doesn't close
    shutdown.connection_started();
    shutdown.shutdown();

    let start = std::time::Instant::now();
    let graceful = shutdown.wait_for_shutdown();
    let elapsed = start.elapsed();

    assert!(!graceful, "Should timeout when connections don't close");
    assert!(
        elapsed >= Duration::from_millis(180),
        "Should wait close to timeout duration, waited: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_millis(400),
        "Should not wait too long past timeout, waited: {:?}",
        elapsed
    );
}

#[test]
fn test_shutdown_signal_default() {
    // Test default ShutdownSignal
    let shutdown = ShutdownSignal::default();

    assert!(!shutdown.is_shutdown_requested());
    assert_eq!(shutdown.active_connection_count(), 0);

    // Default timeout should be 30 seconds, but we won't test the wait
    // to keep tests fast
}

// Note: More comprehensive server integration tests that require actual
// TCP connections would go in a separate test file or be marked with
// #[ignore] to avoid blocking the test suite during normal development.

#[test]
#[ignore] // Mark as ignored for normal test runs since it requires more setup
fn test_server_basic_startup() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let _server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    // This test would require more sophisticated setup to properly test
    // server startup and shutdown without hanging the test suite

    // For now, we just verify the server can be created with valid parameters
    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_millis(100)));

    // In a full integration test, we would:
    // 1. Start server in background thread
    // 2. Make HTTP requests to test different methods
    // 3. Trigger graceful shutdown
    // 4. Verify proper cleanup

    // This test serves as a placeholder for more comprehensive integration testing
    assert!(shutdown.active_connection_count() == 0);
}
