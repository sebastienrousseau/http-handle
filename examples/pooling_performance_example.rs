// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

// examples/pooling_performance_example.rs

//! Performance comparison example demonstrating the benefits of thread pooling
//! and connection pooling under load.
//!
//! This example shows how to use the different server startup methods and
//! demonstrates the performance characteristics of each approach.

use http_handle::{ConnectionPool, Server, ThreadPool};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("🚀 HTTP Handle - Pooling Performance Example");
    println!();

    // Demonstrate ThreadPool performance characteristics
    demonstrate_thread_pool_performance();
    println!();

    // Demonstrate ConnectionPool resource management
    demonstrate_connection_pool_management();
    println!();

    // Show server configuration options
    demonstrate_server_configurations();
}

/// Demonstrates the performance benefits of thread pooling
fn demonstrate_thread_pool_performance() {
    println!("📊 Thread Pool Performance Comparison");
    println!("=====================================");

    const NUM_TASKS: usize = 1000;
    const WORK_DURATION_MS: u64 = 5;

    // Test with unlimited thread spawning (simulating original approach)
    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    for _ in 0..NUM_TASKS {
        let counter_clone = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(WORK_DURATION_MS));
            let _ = counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let unlimited_time = start.elapsed();
    println!(
        "⏱️  Unlimited threads: {:?} ({} tasks)",
        unlimited_time, NUM_TASKS
    );

    // Test with thread pool
    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let thread_pool = ThreadPool::new(8); // Reasonable pool size
    let (tx, rx) = std::sync::mpsc::channel();

    for _ in 0..NUM_TASKS {
        let counter_clone = Arc::clone(&counter);
        let tx_clone = tx.clone();
        thread_pool.execute(move || {
            thread::sleep(Duration::from_millis(WORK_DURATION_MS));
            let _ = counter_clone.fetch_add(1, Ordering::SeqCst);
            tx_clone.send(()).unwrap();
        });
    }

    drop(tx); // Close sender to signal completion
    for _ in 0..NUM_TASKS {
        rx.recv().unwrap();
    }

    let pooled_time = start.elapsed();
    println!(
        "⏱️  Thread pool (8):   {:?} ({} tasks)",
        pooled_time, NUM_TASKS
    );
    println!(
        "📈 Performance ratio: {:.2}x",
        unlimited_time.as_secs_f64() / pooled_time.as_secs_f64()
    );
    println!(
        "💡 Thread pool provides better resource utilization and prevents thread exhaustion"
    );
}

/// Demonstrates connection pool resource management
fn demonstrate_connection_pool_management() {
    println!("🔗 Connection Pool Resource Management");
    println!("=====================================");

    let connection_pool = ConnectionPool::new(5);
    println!("🏊 Created connection pool with capacity: 5");

    // Simulate acquiring connections
    let mut guards = Vec::new();

    // Fill the pool to capacity
    for i in 1..=5 {
        match connection_pool.acquire() {
            Ok(guard) => {
                guards.push(guard);
                println!(
                    "✅ Connection {} acquired (active: {})",
                    i,
                    connection_pool.active_count()
                );
            }
            Err(e) => {
                println!("❌ Failed to acquire connection {}: {}", i, e)
            }
        }
    }

    // Try to exceed capacity
    match connection_pool.acquire() {
        Ok(_) => println!("⚠️  Unexpected: Should have been rejected"),
        Err(_) => println!(
            "🛡️  Connection 6 rejected - pool at capacity (active: {})",
            connection_pool.active_count()
        ),
    }

    // Release some connections
    drop(guards.drain(0..2)); // Drop first 2 guards
    println!(
        "🔄 Released 2 connections (active: {})",
        connection_pool.active_count()
    );

    // Should be able to acquire again
    match connection_pool.acquire() {
        Ok(_guard) => println!(
            "✅ New connection acquired after release (active: {})",
            connection_pool.active_count()
        ),
        Err(e) => println!("❌ Failed: {}", e),
    }

    println!(
        "💡 Connection pooling prevents resource exhaustion and enables graceful degradation"
    );
}

/// Shows different server configuration options
fn demonstrate_server_configurations() {
    println!("⚙️  Server Configuration Options");
    println!("================================");

    let _server = Server::new("127.0.0.1:0", ".");

    println!("🔧 Available startup methods:");
    println!(
        "   • server.start()                           - Basic (unlimited threads)"
    );
    println!(
        "   • server.start_with_thread_pool(8)         - Thread pooling"
    );
    println!(
        "   • server.start_with_pooling(8, 100)        - Thread + connection pooling"
    );
    println!(
        "   • server.start_with_graceful_shutdown(30s) - Graceful shutdown"
    );
    println!();

    println!("📋 Recommended configurations:");
    println!("   Development:    start()");
    println!(
        "   Production:     start_with_pooling(num_cpus, max_conns)"
    );
    println!("   High Traffic:   start_with_pooling(16, 1000)");
    println!("   Constrained:    start_with_pooling(4, 50)");
    println!();

    println!("🎯 Performance Guidelines:");
    println!("   • Thread pool size: typically 2x-4x CPU cores");
    println!(
        "   • Max connections: based on memory and file descriptor limits"
    );
    println!("   • Monitor active connections under load");
    println!("   • Use graceful shutdown in production");

    println!();
    println!("Example server with optimal settings:");
    println!("```rust");
    println!("let server = Server::builder()");
    println!("    .address(\"0.0.0.0:8080\")");
    println!("    .document_root(\"/var/www\")");
    println!("    .request_timeout(Duration::from_secs(30))");
    println!("    .build()?;");
    println!();
    println!("// For production workloads");
    println!("server.start_with_pooling(8, 500)?;");
    println!("```");
}
