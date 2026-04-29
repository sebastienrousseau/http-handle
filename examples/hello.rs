// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Minimal usage: `Server::new` over a static document root.
//!
//! Run: `cargo run --example hello`

#[path = "support.rs"]
mod support;

use http_handle::Server;

fn main() {
    support::header("http-handle -- hello");

    support::task_with_output("Construct minimal Server", || {
        let server = Server::new("127.0.0.1:8080", "./public");
        vec![
            format!("address       = {}", server.address()),
            format!(
                "document_root = {}",
                server.document_root().display()
            ),
            "would call: server.start()".to_string(),
        ]
    });

    support::task_with_output("Equivalent ServerBuilder form", || {
        let server = Server::builder()
            .address("127.0.0.1:8080")
            .document_root("./public")
            .build()
            .expect("build");
        vec![
            format!("address       = {}", server.address()),
            format!(
                "document_root = {}",
                server.document_root().display()
            ),
        ]
    });

    support::summary(2);
}
