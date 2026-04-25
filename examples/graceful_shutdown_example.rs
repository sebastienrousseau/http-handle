// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! # Graceful Shutdown Server Example
//!
//! This example demonstrates how to use the `http-handle` library with graceful shutdown
//! capabilities. The server will handle SIGINT (Ctrl+C) and SIGTERM signals gracefully,
//! allowing existing connections to complete before shutting down.
//!
//! ## Usage
//!
//! Run this example and then press Ctrl+C to trigger graceful shutdown:
//!
//! ```bash
//! cargo run --example graceful_shutdown_example
//! ```
//!
//! The server will:
//! 1. Stop accepting new connections
//! 2. Wait for existing connections to complete (up to the timeout)
//! 3. Report the shutdown status

use http_handle::Server;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    println!("🚀 Starting HTTP server with graceful shutdown...");
    println!("📁 Serving files from: ./public");
    println!("🔗 Server URL: http://127.0.0.1:8080");
    println!("⏰ Shutdown timeout: 30 seconds");
    println!();
    println!("Press Ctrl+C to trigger graceful shutdown");
    println!("==========================================");

    let server = Server::new("127.0.0.1:8080", "./public");

    // Start server with 30 second graceful shutdown timeout
    server.start_with_graceful_shutdown(Duration::from_secs(30))
}
