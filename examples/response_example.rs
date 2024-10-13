//! # HTTP Response Example
//!
//! This example demonstrates how to use the `Response` struct from the `http-handle` library
//! to create and send HTTP responses over a TCP stream. The example includes creating a response
//! with status codes, headers, and a body, and then sending it to a client.
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
//! cargo run --example response_example
//! ```

use http_handle::response::Response;
use http_handle::ServerError;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Entry point for the HTTP response example.
///
/// This function sets up a simple TCP listener that accepts a connection and sends an HTTP response
/// back to the client.
fn main() -> Result<(), ServerError> {
    println!("\n🧪 HTTP Response Example\n");

    // Start a TCP listener on localhost:8080
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("✅ Listening on 127.0.0.1:8080");

    // Spawn a thread to simulate a client making a request and ignore the result
    let _ = thread::spawn(|| {
        simulate_client_request();
    });

    // Accept and handle only one connection, then shut down
    if let Some(stream) = listener.incoming().next() {
        match stream {
            Ok(mut stream) => {
                println!("🦀 Server: Connection accepted!");
                handle_client(&mut stream)?;
                println!(
                    "🛑 Shutting down after sending the response."
                );
            }
            Err(e) => {
                println!("❌ Failed to accept connection: {}", e);
            }
        }
    }

    Ok(())
}

/// Handles an incoming TCP stream by sending an HTTP response.
///
/// This function demonstrates the use of the `Response` struct to send an HTTP response
/// with status, headers, and a body.
///
/// # Errors
///
/// Returns a `ServerError` if there is an issue writing to the stream.
fn handle_client(stream: &mut TcpStream) -> Result<(), ServerError> {
    println!("🦀 Server: Handling incoming connection...");

    // Create an HTTP response with a 200 OK status
    let mut response =
        Response::new(200, "OK", b"<h1>Hello, World!</h1>".to_vec());

    // Add headers to the response
    response.add_header("Content-Type", "text/html");
    response
        .add_header("Content-Length", &response.body.len().to_string());

    // Send the response over the stream
    if let Err(e) = response.send(stream) {
        match e {
            ServerError::Io(ref err)
                if err.kind() == ErrorKind::BrokenPipe =>
            {
                println!("❗ Server: Client disconnected (Broken pipe), but response was sent successfully.");
            }
            _ => return Err(e),
        }
    } else {
        println!("✅ Server: Successfully sent response: 200 OK");
    }

    Ok(())
}

/// Simulates sending an HTTP request to the server.
///
/// This function connects to the local server and sends a basic HTTP GET request.
fn simulate_client_request() {
    // Give the server a moment to start up
    thread::sleep(std::time::Duration::from_millis(1000));

    // Connect to the server
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
        println!("🌐 Client: Simulating HTTP GET request...");
        let request = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        stream
            .write_all(request)
            .expect("Client: Failed to write request");

        // Ensure the request is fully sent by flushing the stream
        stream.flush().expect("Client: Failed to flush the stream");

        // Read and display the response from the server
        let mut buffer = [0; 512];
        let _ = stream
            .read(&mut buffer)
            .expect("Client: Failed to read response");
        println!(
            "🌐 Client: Received response:\n{}",
            String::from_utf8_lossy(&buffer)
        );
    } else {
        println!("❌ Client: Failed to connect to server");
    }
}
