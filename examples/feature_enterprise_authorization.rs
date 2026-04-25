// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle
//! Demonstrates enterprise authorization policy enforcement for HTTP requests.

#[cfg(feature = "enterprise")]
use http_handle::enterprise::{
    AuthorizationHook, RbacAdapter, enforce_http_request_authorization,
};
#[cfg(feature = "enterprise")]
use http_handle::request::Request;

#[cfg(feature = "enterprise")]
fn main() {
    let auth = AuthorizationHook::new().with_engine(
        RbacAdapter::default()
            .grant_role("service-a", "reader")
            .grant_permission("reader", "/health", "GET"),
    );

    let request = Request {
        method: "GET".to_string(),
        path: "/health".to_string(),
        version: "HTTP/1.1".to_string(),
        headers: Vec::new(),
    };

    enforce_http_request_authorization(
        &auth,
        &request,
        "service-a",
        std::collections::HashMap::new(),
    )
    .expect("request should be authorized");

    println!("Enterprise authorization request check passed.");
}

#[cfg(not(feature = "enterprise"))]
fn main() {
    eprintln!("Enable the 'enterprise' feature to run this example.");
}
