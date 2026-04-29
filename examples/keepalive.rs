// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! HTTP/1.1 keep-alive: serve N requests on a single TCP connection.
//!
//! Run: `cargo run --example keepalive`

#[path = "support.rs"]
mod support;

use http_handle::Server;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    support::header("http-handle -- keepalive");

    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("index.html"), b"<h1>ok</h1>")
        .expect("seed index.html");
    std::fs::create_dir(dir.path().join("404")).expect("404 dir");
    std::fs::write(dir.path().join("404/index.html"), b"404")
        .expect("seed 404");

    let probe =
        std::net::TcpListener::bind("127.0.0.1:0").expect("probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let server = Server::builder()
        .address(&addr)
        .document_root(dir.path().to_string_lossy().as_ref())
        .request_timeout(Duration::from_secs(2))
        .build()
        .expect("build");

    // Run the listener on a worker thread; the main thread drives
    // requests over a single keep-alive connection. `stop` is held
    // for the whole demo to keep the server alive long enough.
    let stop = Arc::new(AtomicBool::new(false));
    let stop_in = Arc::clone(&stop);
    let server_thread = thread::spawn(move || {
        // start_with_thread_pool is non-blocking on accept errors;
        // we drop it once `stop` flips by exiting the binary.
        let _ = stop_in;
        let _ = server.start_with_thread_pool(2);
    });

    // Wait for bind.
    for _ in 0..50 {
        if TcpStream::connect(&addr).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    support::task_with_output(
        "5 GETs on a single TCP connection (keep-alive)",
        || {
            let mut stream =
                TcpStream::connect(&addr).expect("connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read_timeout");

            let start = Instant::now();
            let mut succeeded = 0;
            for i in 0..5 {
                let connection =
                    if i == 4 { "close" } else { "keep-alive" };
                let request = format!(
                    "GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: {connection}\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).expect("write");

                // Read just enough to confirm a response framed.
                let mut buf = [0_u8; 256];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]);
                if head.starts_with("HTTP/1.1 200") {
                    succeeded += 1;
                }
            }
            let elapsed = start.elapsed();

            vec![
                format!("addr        = {addr}"),
                format!("requests    = 5 (5th sends Connection: close)"),
                format!("status 200  = {succeeded} / 5"),
                format!("elapsed     = {elapsed:?}"),
                "Each request reused the same TCP connection — only the".into(),
                "fifth `Connection: close` triggers a graceful peer shutdown.".into(),
            ]
        },
    );

    support::task_with_output(
        "Server config controlling keep-alive behaviour",
        || {
            vec![
                "server.MAX_KEEPALIVE_REQUESTS = 100   // per-connection request cap".into(),
                "server.KEEPALIVE_IDLE_TIMEOUT = 5s    // idle window between requests".into(),
                "request_timeout(Duration)             // first request only".into(),
            ]
        },
    );

    stop.store(true, Ordering::SeqCst);
    drop(server_thread); // detached; process exit cleans up.
    support::summary(2);
}
