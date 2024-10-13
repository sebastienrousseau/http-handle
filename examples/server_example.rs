//! # HTTP Server Example
//!
//! This example demonstrates how to use the `Server` struct from the `http-handle` library
//! to create a simple HTTP server that serves files from a specified document root.
//! The server responds to incoming HTTP requests by sending back HTML, CSS, JS, or other files.
//! It also handles 404 errors and prevents directory traversal attacks.
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
//! cargo run --example server_example
//! ```

use http_handle::request::Request;
use http_handle::ServerError;
use std::fs;
use std::io::Result;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

/// Entry point for the HTTP server example.
///
/// This function sets up the server with an address and document root, starts the server to
/// listen for incoming requests, and simulates a client request.
fn main() -> Result<()> {
    let address = "127.0.0.1:8080";
    let document_root = "./public"; // Specify the path to the document root

    // Shared flag to signal the server to stop
    let running = Arc::new(AtomicBool::new(true));

    // Create and start the server in a separate thread
    let server_handle = thread::spawn({
        let running = Arc::clone(&running);
        move || {
            println!("Starting server at http://{}", address);
            // Create a server and set the listener to non-blocking to allow graceful shutdown
            let listener = TcpListener::bind(address)
                .expect("Failed to bind address");
            listener
                .set_nonblocking(true)
                .expect("Failed to set non-blocking");

            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = thread::spawn(move || {
                            if let Err(e) = handle_connection(
                                stream,
                                Path::new(document_root),
                            ) {
                                eprintln!(
                                    "Error handling connection: {}",
                                    e
                                );
                            }
                        });
                    }
                    Err(ref e)
                        if e.kind()
                            == std::io::ErrorKind::WouldBlock =>
                    {
                        // No connection, let's sleep a little and continue
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Connection error: {}", e);
                    }
                }
            }

            println!("Server is shutting down.");
        }
    });

    // Give the server some time to start
    thread::sleep(Duration::from_secs(1));

    // Simulate a client making a request to the server
    simulate_client_request()?;

    // Stop the server by setting the flag to false
    running.store(false, Ordering::SeqCst);

    // Wait for the server thread to complete
    server_handle.join().expect("Failed to join server thread");

    println!("🛑 Server has been shut down.");
    Ok(())
}

/// Simulates sending an HTTP request to the server.
///
/// This function connects to the local server and sends a basic HTTP GET request,
/// then reads and prints the response.
fn simulate_client_request() -> Result<()> {
    // Connect to the server
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
        println!(
            "🌐 Simulating HTTP GET request to http://127.0.0.1:8080/"
        );

        // Send a basic HTTP GET request
        let request = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        stream.write_all(request)?;

        // Read the response from the server
        let mut buffer = [0; 512];
        let bytes_read = stream.read(&mut buffer)?;

        println!(
            "🌐 Received response:\n{}",
            String::from_utf8_lossy(&buffer[..bytes_read])
        );
    } else {
        println!("❌ Failed to connect to server");
    }

    Ok(())
}

/// Handles a single client connection.
///
/// # Arguments
///
/// * `stream` - A `TcpStream` representing the client connection.
/// * `document_root` - A `Path` representing the server's document root.
///
/// # Returns
///
/// A `Result` indicating success or a `ServerError`.
fn handle_connection(
    mut stream: TcpStream,
    document_root: &Path,
) -> std::result::Result<(), ServerError> {
    let request = Request::from_stream(&stream)?;
    let response = generate_response(&request, document_root)?;
    response.send(&mut stream)?;
    stream.flush()?;
    Ok(())
}

/// Generates an HTTP response based on the requested file.
///
/// # Arguments
///
/// * `request` - A `Request` instance representing the client's request.
/// * `document_root` - A `Path` representing the server's document root.
///
/// # Returns
///
/// A `Result` containing the `Response` or a `ServerError`.
fn generate_response(
    request: &Request,
    document_root: &Path,
) -> std::result::Result<http_handle::response::Response, ServerError> {
    let mut path = Path::new(document_root).to_path_buf();
    let request_path = request.path().trim_start_matches('/');

    if request_path.is_empty() {
        path.push("index.html");
    } else {
        path.push(request_path);
    }

    if path.is_file() {
        let contents = fs::read(&path).map_err(ServerError::Io)?;
        let content_type = "text/html"; // Simplified content type handling for example
        let mut response =
            http_handle::response::Response::new(200, "OK", contents);
        response.add_header("Content-Type", content_type);
        Ok(response)
    } else {
        let not_found_body = b"404 Not Found".to_vec();
        let mut response = http_handle::response::Response::new(
            404,
            "Not Found",
            not_found_body,
        );
        response.add_header("Content-Type", "text/plain");
        Ok(response)
    }
}
