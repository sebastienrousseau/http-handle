// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `high-perf` feature: async-first server with backpressure + sendfile.
//!
//! Run: `cargo run --example perf --features high-perf`

#[cfg(feature = "high-perf")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "high-perf")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    use http_handle::Server;
    use http_handle::perf_server::{PerfLimits, start_high_perf};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use tokio::time::{Duration, sleep};

    support::header("http-handle -- perf");

    let probe =
        std::net::TcpListener::bind("127.0.0.1:0").expect("probe");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("index.html"), b"hello-perf")
        .expect("seed");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("404 page");

    let limits = PerfLimits {
        max_inflight: 64,
        max_queue: 256,
        sendfile_threshold_bytes: 64 * 1024,
    };

    let server = Server::builder()
        .address(&addr)
        .document_root(root.path().to_str().expect("path"))
        .build()
        .expect("build");

    let task = tokio::spawn(start_high_perf(server, limits));
    sleep(Duration::from_millis(60)).await;

    support::task_with_output(
        "PerfLimits caps inflight + queue, gates sendfile fast-path",
        || {
            vec![
                format!(
                    "max_inflight             = {}",
                    limits.max_inflight
                ),
                format!(
                    "max_queue                = {}",
                    limits.max_queue
                ),
                format!(
                    "sendfile_threshold_bytes = {}",
                    limits.sendfile_threshold_bytes
                ),
            ]
        },
    );

    support::task_with_output(
        "Single GET roundtrip against the high-perf accept loop",
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

    task.abort();
    support::summary(2);
}

#[cfg(not(feature = "high-perf"))]
fn main() {
    eprintln!(
        "Enable the 'high-perf' feature: cargo run --example perf --features high-perf"
    );
}
