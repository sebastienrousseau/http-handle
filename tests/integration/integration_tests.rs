// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Integration tests for HTTP methods and graceful shutdown scenarios
//!
//! These tests verify the server's behavior with real TCP connections
//! and various HTTP methods including HEAD, OPTIONS, and graceful shutdown.

use http_handle::{Server, ShutdownSignal};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Helper function to find an available port for testing
fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

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

    // Create a CSS file
    fs::write(root_path.join("style.css"), "body { color: red; }")
        .unwrap();

    temp_dir
}

/// Helper function to make an HTTP request and return the response
fn make_http_request(address: &str, request: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();

    let mut response = String::new();
    let _bytes_read = stream
        .read_to_string(&mut response)
        .expect("Failed to read response");
    response
}

/// Helper function to parse HTTP response and extract status line and headers
fn parse_response(response: &str) -> (String, Vec<String>, String) {
    let mut lines = response.lines();
    let status_line = lines.next().unwrap_or("").to_string();

    let mut headers = Vec::new();
    let mut body_started = false;
    let mut body_lines = Vec::new();

    for line in lines {
        if line.is_empty() && !body_started {
            body_started = true;
        } else if body_started {
            body_lines.push(line);
        } else {
            headers.push(line.to_string());
        }
    }

    let body = body_lines.join("\n");
    (status_line, headers, body)
}

#[test]
fn test_server_get_request() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    // Start server in background thread
    let _server_handle = thread::spawn(move || {
        // Only run for a short time
        if let Err(e) = server.start() {
            println!("Server error: {}", e);
        }
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Make GET request
    let request = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = make_http_request(&address, request);

    let (status_line, _headers, body) = parse_response(&response);
    assert!(
        status_line.contains("200 OK"),
        "Expected 200 OK, got: {}",
        status_line
    );
    assert!(
        body.contains("Hello, Integration Test!"),
        "Expected body content, got: {}",
        body
    );

    // Note: In a real test, we'd need a way to gracefully shut down the server
    // For now, this test validates the basic GET functionality
}

#[test]
fn test_server_head_request() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    // Create a shutdown signal for controlled server lifecycle
    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_secs(2)));
    let shutdown_clone = shutdown.clone();

    // Start server with shutdown signal
    let server_handle = thread::spawn(move || {
        if let Err(e) =
            server.start_with_shutdown_signal(shutdown_clone)
        {
            println!("Server error: {}", e);
        }
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Make HEAD request
    let request = "HEAD / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = make_http_request(&address, request);

    let (status_line, headers, body) = parse_response(&response);

    // Verify HEAD response characteristics
    assert!(
        status_line.contains("200 OK"),
        "Expected 200 OK, got: {}",
        status_line
    );
    assert!(
        body.is_empty(),
        "HEAD response should have empty body, got: {}",
        body
    );

    // Verify Content-Length header is present
    let has_content_length = headers
        .iter()
        .any(|h| h.to_lowercase().starts_with("content-length:"));
    assert!(
        has_content_length,
        "HEAD response should include Content-Length header"
    );

    // Gracefully shutdown server
    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_server_options_request() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_secs(2)));
    let shutdown_clone = shutdown.clone();

    let server_handle = thread::spawn(move || {
        if let Err(e) =
            server.start_with_shutdown_signal(shutdown_clone)
        {
            println!("Server error: {}", e);
        }
    });

    thread::sleep(Duration::from_millis(100));

    // Make OPTIONS request
    let request = "OPTIONS / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = make_http_request(&address, request);

    let (status_line, headers, body) = parse_response(&response);

    // Verify OPTIONS response characteristics
    assert!(
        status_line.contains("200 OK"),
        "Expected 200 OK, got: {}",
        status_line
    );
    assert!(
        body.is_empty(),
        "OPTIONS response should have empty body, got: {}",
        body
    );

    // Verify Allow header is present with correct methods
    let allow_header = headers
        .iter()
        .find(|h| h.to_lowercase().starts_with("allow:"))
        .expect("OPTIONS response should include Allow header");

    assert!(
        allow_header.to_lowercase().contains("get"),
        "Allow header should include GET"
    );
    assert!(
        allow_header.to_lowercase().contains("head"),
        "Allow header should include HEAD"
    );
    assert!(
        allow_header.to_lowercase().contains("options"),
        "Allow header should include OPTIONS"
    );

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_server_unsupported_method() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_secs(2)));
    let shutdown_clone = shutdown.clone();

    let server_handle = thread::spawn(move || {
        if let Err(e) =
            server.start_with_shutdown_signal(shutdown_clone)
        {
            println!("Server error: {}", e);
        }
    });

    thread::sleep(Duration::from_millis(100));

    // Make POST request (unsupported method)
    let request = "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let response = make_http_request(&address, request);

    let (status_line, headers, body) = parse_response(&response);

    // Verify 405 Method Not Allowed response
    assert!(
        status_line.contains("405"),
        "Expected 405 Method Not Allowed, got: {}",
        status_line
    );
    assert!(
        body.contains("Method Not Allowed"),
        "Expected method not allowed message in body"
    );

    // Verify Allow header is present
    let allow_header = headers
        .iter()
        .find(|h| h.to_lowercase().starts_with("allow:"))
        .expect("405 response should include Allow header");

    assert!(
        allow_header.to_lowercase().contains("get"),
        "Allow header should include GET"
    );
    assert!(
        allow_header.to_lowercase().contains("head"),
        "Allow header should include HEAD"
    );
    assert!(
        allow_header.to_lowercase().contains("options"),
        "Allow header should include OPTIONS"
    );

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_graceful_shutdown_signal() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_secs(5)));
    let shutdown_clone = shutdown.clone();

    let start_time = Instant::now();
    let server_handle = thread::spawn(move || {
        server.start_with_shutdown_signal(shutdown_clone)
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Trigger shutdown
    shutdown.shutdown();

    // Wait for server to shut down
    let result =
        server_handle.join().expect("Server thread should complete");
    let elapsed = start_time.elapsed();

    // Verify shutdown completed successfully
    assert!(
        result.is_ok(),
        "Server should shut down without error: {:?}",
        result
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "Shutdown should be quick when no active connections, took: {:?}",
        elapsed
    );
}

#[test]
fn test_graceful_shutdown_with_active_connections() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_secs(3)));
    let shutdown_clone = shutdown.clone();

    let server_handle = thread::spawn(move || {
        server.start_with_shutdown_signal(shutdown_clone)
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Start a connection that will keep the server busy
    let address_clone = address.clone();
    let _connection_handle = thread::spawn(move || {
        // Establish an in-flight request so shutdown has an active connection to wait for.
        if let Ok(mut stream) = TcpStream::connect(&address_clone) {
            let _ = stream.write_all(b"GET / HTTP/1.1");
            thread::sleep(Duration::from_millis(700));
            let _ = stream.write_all(
                b"\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            );

            // Read the response
            let mut response = String::new();
            let _ = stream.read_to_string(&mut response);
        }
    });

    // Brief delay to establish connection
    thread::sleep(Duration::from_millis(200));

    // Trigger shutdown while connection is active
    let start_time = Instant::now();
    shutdown.shutdown();

    // Wait for server to shut down
    let result =
        server_handle.join().expect("Server thread should complete");
    let elapsed = start_time.elapsed();

    // Verify shutdown completed successfully
    assert!(
        result.is_ok(),
        "Server should shut down without error: {:?}",
        result
    );
    assert!(
        elapsed >= Duration::from_millis(400),
        "Shutdown should wait for active connections, took: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "Shutdown should not exceed timeout, took: {:?}",
        elapsed
    );
}

#[test]
fn test_shutdown_signal_methods() {
    // Test ShutdownSignal functionality in isolation
    let shutdown = ShutdownSignal::new(Duration::from_secs(1));

    // Initial state
    assert!(!shutdown.is_shutdown_requested());
    assert_eq!(shutdown.active_connection_count(), 0);

    // Simulate connection lifecycle
    shutdown.connection_started();
    assert_eq!(shutdown.active_connection_count(), 1);

    shutdown.connection_started();
    assert_eq!(shutdown.active_connection_count(), 2);

    shutdown.connection_finished();
    assert_eq!(shutdown.active_connection_count(), 1);

    // Trigger shutdown
    shutdown.shutdown();
    assert!(shutdown.is_shutdown_requested());
    assert_eq!(shutdown.active_connection_count(), 1);

    // Finish remaining connection
    shutdown.connection_finished();
    assert_eq!(shutdown.active_connection_count(), 0);

    // Test wait_for_shutdown with no active connections
    let graceful = shutdown.wait_for_shutdown();
    assert!(
        graceful,
        "Should shut down gracefully when no active connections"
    );
}

#[test]
fn test_shutdown_timeout_behavior() {
    // Test timeout behavior when connections don't drain
    let shutdown = ShutdownSignal::new(Duration::from_millis(500));

    // Simulate a connection that doesn't finish
    shutdown.connection_started();
    shutdown.shutdown();

    let start_time = Instant::now();
    let graceful = shutdown.wait_for_shutdown();
    let elapsed = start_time.elapsed();

    // Should timeout and return false
    assert!(
        !graceful,
        "Should not shut down gracefully when connections remain"
    );
    assert!(
        elapsed >= Duration::from_millis(450),
        "Should wait at least close to the timeout duration, waited: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_millis(800),
        "Should not wait too long past timeout, waited: {:?}",
        elapsed
    );
}
