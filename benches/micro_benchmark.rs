// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

#![allow(missing_docs)]

//! Sub-microsecond micro-benchmarks for the public hot-path API.
//!
//! The integration benches in `server_benchmark.rs` measure full TCP
//! roundtrips, where per-call wins under ~50 ns disappear into the
//! kernel-syscall + scheduler noise floor (criterion typically reports
//! ~50 µs noise at 8-way concurrency on Apple Silicon). This file
//! pins the in-process functions criterion can actually resolve at
//! that resolution:
//!
//! * `request_header_lookup_*` — `Request::header(name)` linear-scan
//!   over a `Vec<(String, String)>` at 4, 16, 64 header counts. Lands
//!   the P1.A win against future regressions on the lookup path.
//! * `response_set_connection_header` — `Response::set_connection_header`
//!   replaces an existing `Connection` header in-place. Pinpoints the
//!   keep-alive policy write that runs on every response.
//!
//! The ETag cache (P1.C) and sharded rate limiter (P1.B) live behind
//! private functions in `src/server.rs`; their wins are measured via
//! the integration benches `sync_server_rate_limit_concurrent_8` and
//! the file-serve roundtrips respectively.

use criterion::{Criterion, criterion_group, criterion_main};
use http_handle::request::Request;
use http_handle::response::Response;
use std::hint::black_box;
use std::time::Duration;

fn build_request(header_count: usize) -> Request {
    let headers = (0..header_count)
        .map(|i| (format!("x-test-header-{i}"), format!("value-{i}")))
        .collect();
    Request {
        method: "GET".to_string(),
        path: "/".to_string(),
        version: "HTTP/1.1".to_string(),
        headers,
    }
}

fn bench_request_header_lookup_4(c: &mut Criterion) {
    let request = build_request(4);
    let _ = c.bench_function("request_header_lookup_4_headers", |b| {
        b.iter(|| {
            // Lookup a header that exists at the end of the Vec — the
            // worst-case linear scan. `header()` is case-insensitive
            // so the bench also exercises eq_ignore_ascii_case.
            let v = request.header(black_box("X-Test-Header-3"));
            let _ = black_box(v);
        });
    });
}

fn bench_request_header_lookup_16(c: &mut Criterion) {
    let request = build_request(16);
    let _ = c.bench_function("request_header_lookup_16_headers", |b| {
        b.iter(|| {
            let v = request.header(black_box("X-Test-Header-15"));
            let _ = black_box(v);
        });
    });
}

fn bench_request_header_lookup_64(c: &mut Criterion) {
    let request = build_request(64);
    let _ = c.bench_function("request_header_lookup_64_headers", |b| {
        b.iter(|| {
            let v = request.header(black_box("X-Test-Header-63"));
            let _ = black_box(v);
        });
    });
}

fn bench_response_set_connection_header(c: &mut Criterion) {
    let _ = c.bench_function(
        "response_set_connection_header_replace",
        |b| {
            b.iter(|| {
                // Build a fresh response inside the iter so each call
                // does a real `retain` over an existing Connection
                // header — the actual hot path on every keep-alive
                // response.
                let mut response =
                    Response::new(200, "OK", b"".to_vec());
                response.add_header("Content-Type", "text/html");
                response.add_header("ETag", "W/\"1f-68a0cf20\"");
                response.add_header("Connection", "close");
                response.add_header("Accept-Ranges", "bytes");
                response.set_connection_header(black_box("keep-alive"));
                let _ = black_box(response);
            });
        },
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .sample_size(200);
    targets =
        bench_request_header_lookup_4,
        bench_request_header_lookup_16,
        bench_request_header_lookup_64,
        bench_response_set_connection_header
}
criterion_main!(benches);
