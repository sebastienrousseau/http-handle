// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `ThreadPool` and `ConnectionPool`: bounded resources for production.
//!
//! Run: `cargo run --example pool`

#[path = "support.rs"]
mod support;

use http_handle::{ConnectionPool, ThreadPool};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    support::header("http-handle -- pool");

    support::task_with_output(
        "ThreadPool throughput vs unbounded thread::spawn",
        || {
            const N: usize = 1000;
            const WORK_MS: u64 = 5;

            // Unbounded: one OS thread per task.
            let counter = Arc::new(AtomicUsize::new(0));
            let start = Instant::now();
            let mut handles = Vec::with_capacity(N);
            for _ in 0..N {
                let c = Arc::clone(&counter);
                handles.push(thread::spawn(move || {
                    thread::sleep(Duration::from_millis(WORK_MS));
                    let _ = c.fetch_add(1, Ordering::SeqCst);
                }));
            }
            for h in handles {
                h.join().expect("join");
            }
            let unbounded = start.elapsed();

            // 8-worker pool reused across N tasks.
            let counter = Arc::new(AtomicUsize::new(0));
            let pool = ThreadPool::new(8);
            let (tx, rx) = mpsc::channel();
            let start = Instant::now();
            for _ in 0..N {
                let c = Arc::clone(&counter);
                let tx = tx.clone();
                pool.execute(move || {
                    thread::sleep(Duration::from_millis(WORK_MS));
                    let _ = c.fetch_add(1, Ordering::SeqCst);
                    tx.send(()).expect("send");
                });
            }
            drop(tx);
            for _ in 0..N {
                rx.recv().expect("recv");
            }
            let pooled = start.elapsed();

            vec![
                format!("unbounded = {:.0?} ({} tasks)", unbounded, N),
                format!("pooled-8  = {:.0?} ({} tasks)", pooled, N),
                format!(
                    "ratio     = {:.2}x",
                    unbounded.as_secs_f64() / pooled.as_secs_f64()
                ),
            ]
        },
    );

    support::task_with_output(
        "ConnectionPool caps active connections at capacity",
        || {
            let pool = ConnectionPool::new(5);
            let mut guards = Vec::with_capacity(5);
            for _ in 1..=5 {
                guards.push(pool.acquire().expect("acquire"));
            }
            let rejected = pool.acquire().is_err();
            drop(guards.drain(0..2));
            let after_release = pool.active_count();
            vec![
                "capacity      = 5".into(),
                format!(
                    "filled        = {} / 5",
                    pool.active_count() + 2
                ),
                format!("6th acquire   = rejected? {rejected}"),
                format!("after 2 drops = {after_release} active"),
            ]
        },
    );

    support::task_with_output(
        "Server start helpers map onto these primitives",
        || {
            vec![
                "server.start()                            // unbounded threads".into(),
                "server.start_with_thread_pool(8)          // bounded workers".into(),
                "server.start_with_pooling(8, 100)         // workers + conn cap".into(),
                "server.start_with_graceful_shutdown(30s)  // signal-aware exit".into(),
            ]
        },
    );

    support::summary(3);
}
