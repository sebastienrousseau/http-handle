// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

#![allow(missing_docs)]

//! Async benchmarks targeting `perf_server::start_high_perf`.
//!
//! Two variants:
//!
//! - `perf_server_async_single` — single-client roundtrip baseline against
//!   the async server. Comparable to `sync_server_small_body_38B`.
//! - `perf_server_async_concurrent_8` — fires 8 in-flight client requests
//!   per criterion iteration. This is the harness that surfaces the
//!   reactor-stall fixes from P0.E (sync `std::fs::*` -> `tokio::fs::*`)
//!   and the lock-free pool fixes from P0.D when run side-by-side.
//!
//! Setup mirrors `server_benchmark.rs`:
//!
//! - Reserves `127.0.0.1:0` via a probe, hands the port to the server.
//! - Reads each response to EOF before dropping the client socket.
//! - Server thread leaks at bench end; the process exits immediately
//!   after Criterion, so the OS reclaims it.

use criterion::{Criterion, criterion_group, criterion_main};
use http_handle::Server;
use http_handle::perf_server::{PerfLimits, start_high_perf};
use std::hint::black_box;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const REQUEST: &[u8] = b"GET /test.html HTTP/1.1\r\nHost: b\r\n\r\n";
const BODY: &[u8] = b"<html><body>Test Content</body></html>";

fn reserve_port() -> (String, TempDir) {
    let probe = TcpListener::bind("127.0.0.1:0").expect("probe bind");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);
    let root = TempDir::new().expect("tempdir");
    (addr, root)
}

fn spawn_perf_server() -> (String, tokio::runtime::Runtime) {
    let (addr, root) = reserve_port();
    std::fs::write(root.path().join("test.html"), BODY)
        .expect("write body");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("write 404");
    let document_root =
        root.path().to_str().expect("utf8 path").to_string();
    let server_addr = addr.clone();

    // Single-threaded runtime is intentional: it makes any reactor-blocking
    // call visible. tokio::fs::* still dispatches to the blocking pool
    // (independent of runtime flavor), so the P0.E fix shows up here as
    // reduced scheduler stalls under the 8-way concurrent variant.
    let _ = thread::spawn(move || {
        let _root_keep = root;
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

    // Wait for bind by attempting connects on a dedicated client runtime.
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
    (addr, client_rt)
}

async fn roundtrip_async(addr: &str) {
    let mut s =
        tokio::net::TcpStream::connect(addr).await.expect("connect");
    s.write_all(REQUEST).await.expect("write");
    let mut sink = Vec::with_capacity(256);
    let _ = s.read_to_end(&mut sink).await.expect("read");
    let _ = black_box(sink);
}

fn bench_perf_server_async_single(c: &mut Criterion) {
    let (addr, rt) = spawn_perf_server();
    let _ = c.bench_function("perf_server_async_single", |b| {
        b.iter(|| {
            rt.block_on(async {
                roundtrip_async(&addr).await;
            });
        });
    });
}

fn drive_async_concurrent(
    rt: &tokio::runtime::Runtime,
    addr: &str,
    parallelism: usize,
) {
    rt.block_on(async {
        let mut tasks = Vec::with_capacity(parallelism);
        for _ in 0..parallelism {
            let addr = addr.to_string();
            tasks.push(tokio::spawn(async move {
                roundtrip_async(&addr).await;
            }));
        }
        for t in tasks {
            let _ = t.await;
        }
    });
}

fn bench_perf_server_async_concurrent_8(c: &mut Criterion) {
    let (addr, rt) = spawn_perf_server();
    let _ = c.bench_function("perf_server_async_concurrent_8", |b| {
        b.iter(|| drive_async_concurrent(&rt, &addr, 8));
    });
}

fn bench_perf_server_async_concurrent_32(c: &mut Criterion) {
    let (addr, rt) = spawn_perf_server();
    let _ = c.bench_function("perf_server_async_concurrent_32", |b| {
        b.iter(|| drive_async_concurrent(&rt, &addr, 32));
    });
}

fn bench_perf_server_async_concurrent_64(c: &mut Criterion) {
    let (addr, rt) = spawn_perf_server();
    let _ = c.bench_function("perf_server_async_concurrent_64", |b| {
        b.iter(|| drive_async_concurrent(&rt, &addr, 64));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(40);
    targets =
        bench_perf_server_async_single,
        bench_perf_server_async_concurrent_8,
        bench_perf_server_async_concurrent_32,
        bench_perf_server_async_concurrent_64
}
criterion_main!(benches);
