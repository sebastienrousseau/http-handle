// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `ServerBuilder`: fluent configuration without starting the listener.
//!
//! Run: `cargo run --example builder`

#[path = "support.rs"]
mod support;

use http_handle::Server;
use std::collections::HashMap;
use std::time::Duration;

fn main() {
    support::header("http-handle -- builder");

    support::task_with_output("CORS-enabled server", || {
        let s = Server::builder()
            .address("127.0.0.1:8081")
            .document_root("examples")
            .enable_cors()
            .cors_origins(vec![
                "https://localhost:3000".into(),
                "https://example.com".into(),
            ])
            .build()
            .expect("build");
        vec![format!("addr = {}", s.address()), "cors = on".into()]
    });

    support::task_with_output("Per-header security hardening", || {
        let _ = Server::builder()
            .address("127.0.0.1:8082")
            .document_root("examples")
            .custom_header("X-Content-Type-Options", "nosniff")
            .custom_header("X-Frame-Options", "DENY")
            .custom_header("X-XSS-Protection", "1; mode=block")
            .build()
            .expect("build");
        vec![
            "X-Content-Type-Options: nosniff".into(),
            "X-Frame-Options: DENY".into(),
            "X-XSS-Protection: 1; mode=block".into(),
        ]
    });

    support::task_with_output(
        "Custom request / connection timeouts",
        || {
            let _ = Server::builder()
                .address("127.0.0.1:8083")
                .document_root("examples")
                .request_timeout(Duration::from_secs(30))
                .connection_timeout(Duration::from_secs(60))
                .build()
                .expect("build");
            vec![
                "request_timeout    = 30s".into(),
                "connection_timeout = 60s".into(),
            ]
        },
    );

    support::task_with_output(
        "Bulk header insertion via HashMap",
        || {
            let mut headers = HashMap::new();
            let _ =
                headers.insert("X-Api-Version".into(), "v1.0".into());
            let _ =
                headers.insert("X-Rate-Limit".into(), "1000".into());
            let _ = Server::builder()
                .address("127.0.0.1:8084")
                .document_root("examples")
                .custom_headers(headers)
                .build()
                .expect("build");
            vec!["headers inserted via HashMap".into()]
        },
    );

    support::task_with_output(
        "CORS toggle (enable then disable)",
        || {
            let _ = Server::builder()
                .address("127.0.0.1:8086")
                .document_root("examples")
                .enable_cors()
                .cors_origins(vec!["https://localhost".into()])
                .disable_cors()
                .build()
                .expect("build");
            vec!["cors = off (disable_cors() wins)".into()]
        },
    );

    let err = support::task(
        "Required-field validation surfaces errors",
        || {
            Server::builder()
                .address("127.0.0.1:8087")
                // missing document_root → must fail
                .build()
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default()
        },
    );
    println!("    \x1b[90merr = {err}\x1b[0m");

    support::summary(6);
}
