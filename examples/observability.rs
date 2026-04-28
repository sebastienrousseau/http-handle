// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `observability` feature: structured tracing via `tracing_subscriber`.
//!
//! Run: `RUST_LOG=info cargo run --example observability --features observability`

#[cfg(feature = "observability")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "observability")]
fn main() {
    use http_handle::observability::init_tracing;
    use tracing::info;

    support::header("http-handle -- observability");

    support::task("Initialise tracing subscriber", || {
        init_tracing();
    });

    support::task("Emit a structured info event", || {
        info!(
            target: "http_handle::example",
            request_id = "demo-001",
            "tracing event from the observability example"
        );
    });

    support::task_with_output(
        "Set RUST_LOG=info (or trace) to see structured output",
        || {
            vec![
                "init_tracing() reads RUST_LOG and pretty-prints fields.".into(),
                "Example: RUST_LOG=http_handle=info,info cargo run --example observability --features observability".into(),
            ]
        },
    );

    support::summary(3);
}

#[cfg(not(feature = "observability"))]
fn main() {
    eprintln!(
        "Enable the 'observability' feature: cargo run --example observability --features observability"
    );
}
