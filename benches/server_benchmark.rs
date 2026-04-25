// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

#![allow(missing_docs)]

//! Criterion benchmarks for the synchronous `Server` hot path and a
//! micro-bench for `Response::send`. The harness:
//!
//! - Binds to `127.0.0.1:0` via a probe listener to discover a free port,
//!   then hands the port off to `Server::start()`. Safe for concurrent runs
//!   (no hard-coded `8082` collision).
//! - Reads every response to EOF before dropping the client socket, so the
//!   previous `Connection reset by peer` noise in stderr is eliminated.
//! - Deliberately uses the blocking accept loop (`Server::start`) rather
//!   than the shutdown-aware loop. The shutdown loop uses a 100ms
//!   sleep-poll on `WouldBlock`, which dominates single-client latency and
//!   is not representative of the hot path.
//!
//! The server thread is leaked at bench end; the process exits immediately
//! after Criterion, so the OS reclaims it.

use criterion::{Criterion, criterion_group, criterion_main};
use http_handle::Server;
use http_handle::response::Response;
use std::hint::black_box;
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Reserves a free TCP port on the loopback interface.
fn reserve_port() -> (String, TempDir) {
    let probe = TcpListener::bind("127.0.0.1:0").expect("probe bind");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);
    let root = TempDir::new().expect("tempdir");
    (addr, root)
}

fn spawn_sync_server(body: &[u8]) -> String {
    let (addr, root) = reserve_port();
    std::fs::write(root.path().join("test.html"), body)
        .expect("write body");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("write 404");
    let document_root =
        root.path().to_str().expect("utf8 path").to_string();
    let addr_for_server = addr.clone();
    // Keep the TempDir alive for the life of the server thread.
    let _ = thread::spawn(move || {
        let _root_keepalive = root;
        let server = Server::new(&addr_for_server, &document_root);
        let _ = server.start();
    });
    // Retry until the kernel reports the port as bound.
    for _ in 0..100 {
        if TcpStream::connect(&addr).is_ok() {
            return addr;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("server never bound on {addr}");
}

fn roundtrip(addr: &str) {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .write_all(b"GET /test.html HTTP/1.1\r\nHost: b\r\n\r\n")
        .expect("write");
    let mut sink = Vec::with_capacity(256);
    let _ = stream.read_to_end(&mut sink).expect("read");
    let _ = black_box(sink);
}

fn bench_sync_server_small_body(c: &mut Criterion) {
    let addr =
        spawn_sync_server(b"<html><body>Test Content</body></html>");
    let _ = c.bench_function("sync_server_small_body_38B", |b| {
        b.iter(|| roundtrip(&addr));
    });
}

fn bench_response_send_small(c: &mut Criterion) {
    let mut response = Response::new(
        200,
        "OK",
        b"<html><body>hello</body></html>".to_vec(),
    );
    response.add_header("Content-Type", "text/html");
    response.add_header("ETag", "W/\"1f-68a0cf20\"");
    response.add_header("Accept-Ranges", "bytes");
    let _ =
        c.bench_function("response_send_small_body_5_headers", |b| {
            b.iter(|| {
                let mut sink =
                    Cursor::new(Vec::<u8>::with_capacity(256));
                response.send(&mut sink).expect("send");
                let _ = black_box(sink.into_inner());
            });
        });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(60);
    targets = bench_sync_server_small_body, bench_response_send_small
}
criterion_main!(benches);
