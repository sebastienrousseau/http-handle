// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Heap-profile harness for the perf_server async path.
//!
//! Run:
//!     cargo run --release --example dhat_alloc_profile --features high-perf
//!
//! Writes `dhat-heap.json` in the current directory. Open it with
//! `dh_view` (<https://nnethercote.github.io/dh_view/dh_view.html>) to
//! get a flamegraph-style breakdown of allocation sites by total bytes,
//! peak bytes, and call count.
//!
//! Workload: 1024 sequential GET /test.html roundtrips against a single
//! `start_high_perf` server bound to a free localhost port. The
//! profile captures the steady-state allocation pattern of the async
//! request → response pipeline; transient setup allocations (tempdir,
//! tokio runtime build) are bracketed by the `Profiler` lifetime so
//! they show up too but in a separately attributable region.
//!
//! This example exists to keep the heap-profiling capability available
//! and reproducible. The numbers it produces are platform- and
//! workload-dependent; treat them as a baseline for "did my refactor
//! make things worse," not as absolute truth.

use http_handle::Server;
use http_handle::perf_server::{PerfLimits, start_high_perf};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const ITERATIONS: usize = 1024;

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    let probe = TcpListener::bind("127.0.0.1:0").expect("probe bind");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);

    let root = tempfile::TempDir::new().expect("tempdir");
    std::fs::write(
        root.path().join("test.html"),
        b"<html><body>Test Content</body></html>",
    )
    .expect("write body");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("write 404");
    let document_root =
        root.path().to_str().expect("utf8 path").to_string();
    let server_addr = addr.clone();

    let _server_thread = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("server rt");
        rt.block_on(async {
            let server = Server::builder()
                .address(&server_addr)
                .document_root(&document_root)
                .build()
                .expect("server build");
            let _ =
                start_high_perf(server, PerfLimits::default()).await;
        });
    });

    // Wait for bind.
    let client_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("client rt");
    client_rt.block_on(async {
        for _ in 0..200 {
            if tokio::net::TcpStream::connect(&addr).await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("perf_server never bound on {addr}");
    });

    client_rt.block_on(async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for _ in 0..ITERATIONS {
            let mut s = tokio::net::TcpStream::connect(&addr)
                .await
                .expect("connect");
            s.write_all(b"GET /test.html HTTP/1.1\r\nHost: b\r\n\r\n")
                .await
                .expect("write");
            let mut sink = Vec::with_capacity(256);
            let _ = s.read_to_end(&mut sink).await.expect("read");
        }
    });

    println!(
        "[dhat_alloc_profile] {ITERATIONS} roundtrips complete; \
         dhat-heap.json written on Profiler drop."
    );
}
