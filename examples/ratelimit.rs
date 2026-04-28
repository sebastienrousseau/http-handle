// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `distributed-rate-limit` feature: shared limiter with in-memory backend.
//!
//! Run: `cargo run --example ratelimit --features distributed-rate-limit`

#[cfg(feature = "distributed-rate-limit")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "distributed-rate-limit")]
fn main() {
    use http_handle::distributed_rate_limit::{
        DistributedRateLimiter, InMemoryBackend,
    };

    support::header("http-handle -- ratelimit");

    support::task_with_output(
        "Two-request budget per minute trips on the third call",
        || {
            let limiter = DistributedRateLimiter::new(
                InMemoryBackend::default(),
                "example",
                2,
                60,
            );
            let source = "127.0.0.1".parse().expect("ip");
            let s1 = limiter.is_limited(source).expect("check");
            let s2 = limiter.is_limited(source).expect("check");
            let s3 = limiter.is_limited(source).expect("check");
            vec![
                format!("call 1 limited? {s1}"),
                format!("call 2 limited? {s2}"),
                format!("call 3 limited? {s3} (over budget)"),
            ]
        },
    );

    support::task_with_output(
        "Distinct sources have independent budgets",
        || {
            let limiter = DistributedRateLimiter::new(
                InMemoryBackend::default(),
                "tenants",
                1,
                60,
            );
            let a = "10.0.0.1".parse().expect("ip");
            let b = "10.0.0.2".parse().expect("ip");
            let a1 = limiter.is_limited(a).expect("check");
            let a2 = limiter.is_limited(a).expect("check");
            let b1 = limiter.is_limited(b).expect("check");
            vec![
                format!("a #1 limited? {a1}"),
                format!("a #2 limited? {a2} (a is now over budget)"),
                format!("b #1 limited? {b1} (b unaffected)"),
            ]
        },
    );

    support::summary(2);
}

#[cfg(not(feature = "distributed-rate-limit"))]
fn main() {
    eprintln!(
        "Enable the 'distributed-rate-limit' feature: cargo run --example ratelimit --features distributed-rate-limit"
    );
}
