// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `high-perf-multi-thread` feature: multi-thread Tokio runtime entry.
//!
//! Run: `cargo run --example multi --features high-perf-multi-thread`

#[cfg(feature = "high-perf-multi-thread")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "high-perf-multi-thread")]
fn main() {
    use http_handle::Server;
    use http_handle::perf_server::{
        PerfLimits, start_high_perf_multi_thread,
    };
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    support::header("http-handle -- multi");

    let probe =
        std::net::TcpListener::bind("127.0.0.1:0").expect("probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("index.html"), b"hello-multi")
        .expect("seed");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("404 page");

    let server = Server::builder()
        .address(&addr)
        .document_root(root.path().to_str().expect("path"))
        .build()
        .expect("build");

    let server_addr = addr.clone();
    let server_thread = thread::spawn(move || {
        // worker_threads = Some(2) pins the runtime to two workers
        // for reproducible demo output. None defaults to logical CPU
        // count (best for production).
        let _ = start_high_perf_multi_thread(
            server,
            PerfLimits::default(),
            Some(2),
        );
        let _ = server_addr;
    });

    // Wait for bind.
    for _ in 0..50 {
        if TcpStream::connect(&addr).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    support::task_with_output(
        "start_high_perf_multi_thread owns the runtime internally",
        || {
            vec![
                "let server = Server::builder()".into(),
                "    .address(\"127.0.0.1:0\")".into(),
                "    .document_root(\"./public\")".into(),
                "    .build()?;".into(),
                "start_high_perf_multi_thread(server, PerfLimits::default(), Some(2))".into(),
                "// callers don't need rt-multi-thread on their tokio dep".into(),
            ]
        },
    );

    support::task_with_output(
        "Single GET hits the multi-thread accept loop",
        || {
            let mut stream =
                TcpStream::connect(&addr).expect("connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read_timeout");
            stream
                .write_all(
                    b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
                )
                .expect("write");
            let mut buffer = [0_u8; 256];
            let read = stream.read(&mut buffer).unwrap_or(0);
            String::from_utf8_lossy(&buffer[..read])
                .lines()
                .take(4)
                .map(str::to_string)
                .collect()
        },
    );

    drop(server_thread); // detached; process exit cleans up.
    support::summary(2);
}

#[cfg(not(feature = "high-perf-multi-thread"))]
fn main() {
    eprintln!(
        "Enable the 'high-perf-multi-thread' feature: cargo run --example multi --features high-perf-multi-thread"
    );
}
