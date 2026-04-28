// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Server policies: CORS, security headers, timeouts, rate limit, cache.
//!
//! Run: `cargo run --example policies`

#[path = "support.rs"]
mod support;

use http_handle::Server;
use std::time::Duration;

fn main() {
    support::header("http-handle -- policies");

    support::task_with_output("Configure full policy stack", || {
        let server = Server::builder()
            .address("127.0.0.1:8080")
            .document_root("./public")
            .enable_cors()
            .cors_origins(vec!["https://example.com".into()])
            .custom_header("X-Frame-Options", "DENY")
            .custom_header("X-Content-Type-Options", "nosniff")
            .request_timeout(Duration::from_secs(15))
            .connection_timeout(Duration::from_secs(30))
            .rate_limit_per_minute(120)
            .static_cache_ttl_secs(300)
            .build()
            .expect("build");
        vec![
            format!("address          = {}", server.address()),
            format!("root             = {}", server.document_root().display()),
            "cors_origins     = [https://example.com]".into(),
            "custom_headers   = X-Frame-Options, X-Content-Type-Options".into(),
            "request_timeout  = 15s".into(),
            "connection_to    = 30s".into(),
            "rate_limit       = 120 / min".into(),
            "cache_ttl        = 300s".into(),
        ]
    });

    support::task_with_output(
        "Cache TTL only (no other policies)",
        || {
            let s = Server::builder()
                .address("127.0.0.1:8090")
                .document_root("./public")
                .static_cache_ttl_secs(60)
                .build()
                .expect("build");
            vec![format!("cache_ttl = 60s on {}", s.address())]
        },
    );

    support::summary(2);
}
