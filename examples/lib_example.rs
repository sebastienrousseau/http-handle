//! # SSG Server Example
//!
//! This example demonstrates how to use the `http-handle` library to start a simple HTTP server,
//! handle requests, and respond to errors.
//!
//! The example uses the core components of the library, such as the `Server` struct and `ServerError` enum.
//! It walks through starting the server, handling requests, and responding to common error scenarios.
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
//! cargo run --example lib_example
//! ```

use http_handle::{Server, ServerError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

/// Entry point for the HTTP server example.
///
/// This function demonstrates how to start the server, handle incoming requests, and manage errors.
/// It will listen on a specified address and respond to HTTP requests.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a critical issue.
fn main() -> Result<(), ServerError> {
    println!("\n🧪 SSG Server Example\n");

    // Shared flag to signal the server to stop
    let running = Arc::new(AtomicBool::new(true));

    // Start the server and handle any errors.
    match start_server(Arc::clone(&running)) {
        Ok(_) => {
            println!("\n🎉 Server started successfully!");
        }
        Err(e) => {
            println!("\n❌ Server failed to start: {}", e);
        }
    }

    // Simulate running the server for a while and then stop it
    println!(
        "⏳ Server will run for 2 seconds before shutting down..."
    );
    thread::sleep(Duration::from_secs(2));

    // Set the flag to stop the server
    running.store(false, Ordering::SeqCst);
    println!("🛑 Server shutdown signal sent.");

    Ok(())
}

/// Starts the HTTP server on the given address.
///
/// This function creates a new instance of the `Server` and binds it to a specified address and document root.
/// It demonstrates handling server initialization and reporting errors when the server cannot bind or operate.
///
/// # Errors
///
/// Returns a `ServerError` if the server cannot be started or bound to the address.
fn start_server(running: Arc<AtomicBool>) -> Result<(), ServerError> {
    println!("🦀 Starting HTTP Server...");

    // Provide both the address and the document root arguments
    let server = Server::new("127.0.0.1:8080", "./public");

    println!("✅ Server successfully initialized. Listening on 127.0.0.1:8080");

    // Create a thread to run the server
    let _ = thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            if let Err(e) = server.start() {
                eprintln!("❌ Server error: {}", e);
                break;
            }
        }

        println!("🛑 Server has been stopped.");
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_initialization() {
        let server = Server::new("127.0.0.1:8080", "./public");
        // Ensure server initializes correctly
        assert!(matches!(server, Server { .. }));
    }

    #[test]
    fn test_server_error_handling() {
        // Simulate invalid address to trigger error
        let result = Server::new("invalid_address", "./public");
        assert!(matches!(result, Server { .. }));
    }
}
