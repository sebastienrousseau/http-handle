// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Graceful shutdown via [`ShutdownSignal`]: drain budget + state.
//!
//! Run: `cargo run --example shutdown`

#[path = "support.rs"]
mod support;

use http_handle::{Server, ShutdownSignal};
use std::time::{Duration, Instant};

fn main() {
    support::header("http-handle -- shutdown");

    support::task_with_output(
        "ShutdownSignal lifecycle (no listener)",
        || {
            let signal = ShutdownSignal::new(Duration::from_secs(30));
            let started = signal.is_shutdown_requested();
            signal.shutdown();
            let triggered = signal.is_shutdown_requested();
            vec![
                format!("initial    = {started}"),
                format!("after .shutdown() = {triggered}"),
                format!(
                    "active_conns      = {}",
                    signal.active_connection_count()
                ),
            ]
        },
    );

    support::task_with_output(
        "Connection counter tracks in-flight work",
        || {
            let signal = ShutdownSignal::new(Duration::from_secs(5));
            signal.connection_started();
            signal.connection_started();
            signal.connection_started();
            let peak = signal.active_connection_count();
            signal.connection_finished();
            let after_one = signal.active_connection_count();
            vec![
                format!("peak       = {peak}"),
                format!("after_drop = {after_one}"),
            ]
        },
    );

    support::task_with_output(
        "wait_for_shutdown returns true when drain finishes within budget",
        || {
            let signal = ShutdownSignal::new(Duration::from_millis(50));
            let start = Instant::now();
            // No active connections registered → drain completes
            // immediately and the call returns true (graceful).
            let graceful = signal.wait_for_shutdown();
            vec![
                format!("graceful = {graceful}"),
                format!("elapsed  ≈ {:?}", start.elapsed()),
            ]
        },
    );

    support::task_with_output(
        "Server::start_with_graceful_shutdown wires a SIGINT-aware loop",
        || {
            let _ = Server::new("127.0.0.1:8080", "./public");
            vec![
                "let server = Server::new(\"127.0.0.1:8080\", \"./public\");".into(),
                "server.start_with_graceful_shutdown(Duration::from_secs(30))".into(),
                "// SIGINT / SIGTERM → stop accepting → drain → exit".into(),
            ]
        },
    );

    support::summary(4);
}
