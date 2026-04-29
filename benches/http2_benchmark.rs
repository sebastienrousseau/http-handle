// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

#![allow(missing_docs)]

//! Criterion bench for the HTTP/2 (h2c) server path.
//!
//! `http2_single_stream_h2c` runs one client request per criterion
//! iteration through the `h2` crate's client end against
//! `start_http2`. Each iter spins a fresh handshake — that's the
//! pessimistic cost; if a future change adds connection pooling /
//! reuse, this bench will swing accordingly.
//!
//! Setup mirrors `perf_server_benchmark.rs`: probe-port discovery,
//! current-thread runtimes, server thread leaked at process exit.

use criterion::{Criterion, criterion_group, criterion_main};
use http_handle::Server;
use http_handle::http2_server::start_http2;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const BODY: &[u8] = b"<html><body>Test Content</body></html>";

fn reserve_port() -> (String, TempDir) {
    let probe = TcpListener::bind("127.0.0.1:0").expect("probe bind");
    let addr = probe.local_addr().expect("addr").to_string();
    drop(probe);
    let root = TempDir::new().expect("tempdir");
    (addr, root)
}

fn spawn_h2_server() -> (String, tokio::runtime::Runtime) {
    let (addr, root) = reserve_port();
    std::fs::write(root.path().join("test.html"), BODY)
        .expect("write body");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("write 404");
    let document_root =
        root.path().to_str().expect("utf8 path").to_string();
    let server_addr = addr.clone();

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
            let _ = start_http2(server).await;
        });
    });

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
        panic!("h2 server never bound on {addr}");
    });
    (addr, client_rt)
}

async fn h2_roundtrip(addr: &str) {
    let stream =
        tokio::net::TcpStream::connect(addr).await.expect("connect");
    let (mut client, connection) =
        h2::client::handshake(stream).await.expect("handshake");
    // Spawn-and-forget the connection driver. Awaiting it after `drop(client)`
    // can deadlock under criterion's tight loop because the driver doesn't
    // exit until the server closes the TCP stream, which may happen after
    // this iter's deadline. The detached task drains naturally.
    drop(tokio::spawn(async move {
        let _ = connection.await;
    }));

    let req = http::Request::builder()
        .method("GET")
        .uri("http://localhost/test.html")
        .body(())
        .expect("request");
    let (response_future, _send) =
        client.send_request(req, true).expect("send request");
    let response = response_future.await.expect("response");
    assert_eq!(response.status().as_u16(), 200);

    let mut body = response.into_body();
    while let Some(chunk) = body.data().await {
        let _ = chunk.expect("chunk");
    }
    drop(client);
}

fn bench_http2_single_stream(c: &mut Criterion) {
    let (addr, rt) = spawn_h2_server();
    let _ = c.bench_function("http2_single_stream_h2c", |b| {
        b.iter(|| {
            rt.block_on(async {
                h2_roundtrip(&addr).await;
            });
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(40);
    targets = bench_http2_single_stream
}
criterion_main!(benches);
