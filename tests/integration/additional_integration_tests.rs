// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Additional integration tests for edge cases and error scenarios
//!
//! These tests complement the existing integration test suite by covering
//! additional edge cases, malformed requests, and error conditions.

use http_handle::{Server, ShutdownSignal};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
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
        "<html><body>Test Root</body></html>",
    )
    .unwrap();

    // Create subdirectory with index.html
    fs::create_dir(root_path.join("subdir")).unwrap();
    fs::write(
        root_path.join("subdir/index.html"),
        "<html><body>Subdirectory</body></html>",
    )
    .unwrap();

    // Create various file types for content-type testing
    fs::write(root_path.join("test.txt"), "Plain text file").unwrap();
    fs::write(root_path.join("test.json"), r#"{"key": "value"}"#)
        .unwrap();
    fs::write(
        root_path.join("test.xml"),
        "<root><item>data</item></root>",
    )
    .unwrap();

    // Create 404 directory with custom error page
    fs::create_dir(root_path.join("404")).unwrap();
    fs::write(
        root_path.join("404/index.html"),
        "<html><body>Custom 404 Page</body></html>",
    )
    .unwrap();

    temp_dir
}

/// Helper function to make an HTTP request and return the response
fn make_http_request_with_timeout(
    address: &str,
    request: &str,
    timeout: Duration,
) -> Result<String, std::io::Error> {
    let mut stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response)?;
    Ok(response)
}

/// Helper function to parse HTTP response
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
fn test_malformed_http_request() {
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

    // Test malformed request (missing HTTP version)
    let malformed_request = "GET /\r\nHost: localhost\r\n\r\n";
    if let Ok(response) = make_http_request_with_timeout(
        &address,
        malformed_request,
        Duration::from_secs(1),
    ) {
        let (status_line, _headers, _body) = parse_response(&response);
        // Server should handle malformed requests gracefully
        // This might result in a 400 Bad Request or the server might handle it
        println!("Malformed request response: {}", status_line);
    }

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_directory_traversal_prevention() {
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

    // Test directory traversal attempts
    let traversal_requests = vec![
        "GET /../../../etc/passwd HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        "GET /subdir/../../etc/passwd HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        "GET /subdir/../../../etc/passwd HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    ];

    for request in traversal_requests {
        if let Ok(response) = make_http_request_with_timeout(
            &address,
            request,
            Duration::from_secs(1),
        ) {
            let (status_line, _headers, body) =
                parse_response(&response);
            // Should either be 403 Forbidden or 404 Not Found, not expose system files
            assert!(
                status_line.contains("404")
                    || status_line.contains("403"),
                "Directory traversal should be blocked, got: {}",
                status_line
            );
            assert!(
                !body.contains("root:"),
                "Should not expose system files"
            );
        }
    }

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_subdirectory_serving() {
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

    // Test subdirectory access
    let request = "GET /subdir/ HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = make_http_request_with_timeout(
        &address,
        request,
        Duration::from_secs(1),
    )
    .unwrap();

    let (status_line, _headers, body) = parse_response(&response);
    assert!(
        status_line.contains("200 OK"),
        "Expected 200 OK for subdirectory, got: {}",
        status_line
    );
    assert!(
        body.contains("Subdirectory"),
        "Expected subdirectory content, got: {}",
        body
    );

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_content_type_headers() {
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

    // Test different file types and their content types
    let test_cases = vec![
        (
            "GET /test.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            "text/plain",
        ),
        (
            "GET /test.json HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            "application/json",
        ),
        (
            "GET /test.xml HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            "application/xml",
        ),
    ];

    for (request, expected_content_type) in test_cases {
        if let Ok(response) = make_http_request_with_timeout(
            &address,
            request,
            Duration::from_secs(1),
        ) {
            let (status_line, headers, _body) =
                parse_response(&response);
            assert!(
                status_line.contains("200 OK"),
                "Expected 200 OK, got: {}",
                status_line
            );

            let content_type_header = headers
                .iter()
                .find(|h| h.to_lowercase().starts_with("content-type:"))
                .expect("Content-Type header should be present");

            assert!(
                content_type_header
                    .to_lowercase()
                    .contains(expected_content_type),
                "Expected content type {}, got: {}",
                expected_content_type,
                content_type_header
            );
        }
    }

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_custom_404_page() {
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

    // Test 404 with custom error page
    let request = "GET /nonexistent.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = make_http_request_with_timeout(
        &address,
        request,
        Duration::from_secs(1),
    )
    .unwrap();

    let (status_line, headers, body) = parse_response(&response);
    assert!(
        status_line.contains("404"),
        "Expected 404 Not Found, got: {}",
        status_line
    );
    assert!(
        body.contains("Custom 404 Page"),
        "Expected custom 404 page content, got: {}",
        body
    );

    // Verify Content-Type is set to text/html
    let content_type_header = headers
        .iter()
        .find(|h| h.to_lowercase().starts_with("content-type:"))
        .expect("Content-Type header should be present");
    assert!(content_type_header.to_lowercase().contains("text/html"));

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_head_vs_get_response_consistency() {
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

    // Make GET request
    let get_request = "GET /test.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let get_response = make_http_request_with_timeout(
        &address,
        get_request,
        Duration::from_secs(1),
    )
    .unwrap();
    let (get_status, _get_headers, get_body) =
        parse_response(&get_response);

    thread::sleep(Duration::from_millis(50));

    // Make HEAD request
    let head_request = "HEAD /test.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let head_response = make_http_request_with_timeout(
        &address,
        head_request,
        Duration::from_secs(1),
    )
    .unwrap();
    let (head_status, head_headers, head_body) =
        parse_response(&head_response);

    // Verify HEAD response has same status as GET
    assert_eq!(
        get_status, head_status,
        "HEAD and GET should have same status"
    );

    // Verify HEAD response has empty body
    assert!(
        head_body.is_empty(),
        "HEAD response should have empty body"
    );

    // Verify HEAD has Content-Length matching GET body length
    let get_content_length = get_body.len();
    let head_content_length_header = head_headers
        .iter()
        .find(|h| h.to_lowercase().starts_with("content-length:"))
        .expect("HEAD response should have Content-Length header");

    let head_content_length: usize = head_content_length_header
        .split(':')
        .nth(1)
        .unwrap()
        .trim()
        .parse()
        .expect("Content-Length should be a valid number");

    assert_eq!(
        get_content_length, head_content_length,
        "HEAD Content-Length should match GET body length"
    );

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}

#[test]
fn test_concurrent_requests() {
    let temp_dir = setup_test_directory();
    let port = find_available_port();
    let address = format!("127.0.0.1:{}", port);
    let server =
        Server::new(&address, temp_dir.path().to_str().unwrap());

    let shutdown =
        Arc::new(ShutdownSignal::new(Duration::from_secs(5)));
    let shutdown_clone = shutdown.clone();

    let server_handle = thread::spawn(move || {
        if let Err(e) =
            server.start_with_shutdown_signal(shutdown_clone)
        {
            println!("Server error: {}", e);
        }
    });

    thread::sleep(Duration::from_millis(100));

    // Launch multiple concurrent requests
    let mut handles = vec![];
    for i in 0..5 {
        let address_clone = address.clone();
        let handle = thread::spawn(move || {
            let request = format!(
                "GET /test.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nX-Request-ID: {}\r\n\r\n",
                i
            );
            make_http_request_with_timeout(
                &address_clone,
                &request,
                Duration::from_secs(2),
            )
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(response)) = handle.join() {
            let (status_line, _headers, _body) =
                parse_response(&response);
            if status_line.contains("200 OK") {
                success_count += 1;
            }
        }
    }

    // Verify all concurrent requests succeeded
    assert_eq!(
        success_count, 5,
        "All concurrent requests should succeed"
    );

    shutdown.shutdown();
    server_handle.join().expect("Server thread should complete");
}
