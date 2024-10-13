//! # HTTP Request Example
//!
//! This example demonstrates how to use the `Request` struct from the `http-handle` library
//! to parse HTTP requests from a TCP stream. The example includes creating and handling
//! incoming HTTP requests, validating them, and managing various error conditions.
//!
//! ## Usage
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! http-handle = "0.1"
//! ```
//!
//! Then, run the example using:
//! ```sh
//! cargo run --example request_example
//! ```

use http_handle::request::Request;
use http_handle::ServerError;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Entry point for the request parsing example.
///
/// This function sets up a simple TCP listener that accepts a connection, reads the HTTP request,
/// and prints the parsed request or an error message. It also simulates sending an HTTP request.
fn main() -> Result<(), ServerError> {
    println!("\n🧪 HTTP Request Parsing Example\n");

    // Start a TCP listener on localhost:8080 in a separate thread
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("✅ Listening on 127.0.0.1:8080");

    // Spawn a thread to simulate a client sending an HTTP request and join it to wait for it to finish.
    let handle = thread::spawn(|| {
        simulate_http_request();
    });

    // Wait for the simulated client thread to finish.
    handle.join().expect("Failed to join thread");

    // Accept and handle only one connection, then shut down
    if let Some(stream) = listener.incoming().next() {
        match stream {
            Ok(stream) => {
                handle_client(stream)?;
                println!("🛑 Shutting down after handling the first request.");
            }
            Err(e) => {
                println!("❌ Failed to accept connection: {}", e);
            }
        }
    }

    Ok(())
}

/// Handles an incoming TCP stream by parsing the HTTP request.
///
/// This function demonstrates the use of the `Request::from_stream` method to parse an HTTP request
/// and print it. If the request is invalid, it prints an error message.
///
/// # Errors
///
/// Returns a `ServerError` if there is an issue reading or parsing the request.
fn handle_client(stream: TcpStream) -> Result<(), ServerError> {
    println!("🦀 Handling incoming connection...");

    // Parse the HTTP request from the stream
    match Request::from_stream(&stream) {
        Ok(request) => {
            println!("✅ Successfully parsed request: {}", request);
        }
        Err(e) => {
            println!("❌ Error parsing request: {}", e);
        }
    }

    Ok(())
}

/// Simulates sending an HTTP request to the server.
///
/// This function connects to the local server and sends a simple GET request.
fn simulate_http_request() {
    // Give the server a moment to start up
    thread::sleep(std::time::Duration::from_millis(500));

    // Connect to the server
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
        println!("🌐 Simulating HTTP GET request...");
        let request = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        stream.write_all(request).expect("Failed to write request");
    } else {
        println!("❌ Failed to connect to server");
    }
}
