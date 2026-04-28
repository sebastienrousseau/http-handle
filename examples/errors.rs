// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `ServerError` constructors: I/O wrap, semantic helpers, custom.
//!
//! Run: `cargo run --example errors`

#[path = "support.rs"]
mod support;

use http_handle::ServerError;

fn main() {
    support::header("http-handle -- errors");

    support::task_with_output("Wrap a std::io::Error", || {
        let inner = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        );
        let err = ServerError::from(inner);
        vec![format!("{err}")]
    });

    support::task_with_output("Invalid request", || {
        let err = ServerError::invalid_request("Missing HTTP method");
        vec![format!("{err}")]
    });

    support::task_with_output("Not found", || {
        let err = ServerError::not_found("/nonexistent.html");
        vec![format!("{err}")]
    });

    support::task_with_output("Forbidden", || {
        let err =
            ServerError::forbidden("Access denied to sensitive file");
        vec![format!("{err}")]
    });

    support::task_with_output("Custom error from string", || {
        let err: ServerError = "Unexpected error".into();
        vec![format!("{err}")]
    });

    support::summary(5);
}
