// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `enterprise` feature: RBAC adapter + per-request enforcement hook.
//!
//! Run: `cargo run --example enterprise --features enterprise`

#[cfg(feature = "enterprise")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "enterprise")]
fn main() {
    use http_handle::enterprise::{
        AuthorizationHook, RbacAdapter,
        enforce_http_request_authorization,
    };
    use http_handle::request::Request;
    use std::collections::HashMap;

    support::header("http-handle -- enterprise");

    let auth = AuthorizationHook::new().with_engine(
        RbacAdapter::default()
            .grant_role("service-a", "reader")
            .grant_role("service-b", "writer")
            .grant_permission("reader", "/health", "GET")
            .grant_permission("writer", "/health", "GET")
            .grant_permission("writer", "/admin", "POST"),
    );

    support::task_with_output(
        "Authorise GET /health for the reader role",
        || {
            let request = Request {
                method: "GET".into(),
                path: "/health".into(),
                version: "HTTP/1.1".into(),
                headers: Vec::new(),
            };
            let outcome = enforce_http_request_authorization(
                &auth,
                &request,
                "service-a",
                HashMap::new(),
            );
            vec![format!(
                "service-a GET /health -> {:?}",
                outcome.map(|_| "allowed")
            )]
        },
    );

    support::task_with_output(
        "Reject POST /admin for the reader role",
        || {
            let request = Request {
                method: "POST".into(),
                path: "/admin".into(),
                version: "HTTP/1.1".into(),
                headers: Vec::new(),
            };
            let outcome = enforce_http_request_authorization(
                &auth,
                &request,
                "service-a",
                HashMap::new(),
            );
            vec![format!(
                "service-a POST /admin -> err? {}",
                outcome.is_err()
            )]
        },
    );

    support::task_with_output(
        "Allow POST /admin for the writer role",
        || {
            let request = Request {
                method: "POST".into(),
                path: "/admin".into(),
                version: "HTTP/1.1".into(),
                headers: Vec::new(),
            };
            let outcome = enforce_http_request_authorization(
                &auth,
                &request,
                "service-b",
                HashMap::new(),
            );
            vec![format!(
                "service-b POST /admin -> {:?}",
                outcome.map(|_| "allowed")
            )]
        },
    );

    support::summary(3);
}

#[cfg(not(feature = "enterprise"))]
fn main() {
    eprintln!(
        "Enable the 'enterprise' feature: cargo run --example enterprise --features enterprise"
    );
}
