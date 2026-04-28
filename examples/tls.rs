// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `tls` (via `enterprise` umbrella): TLS / mTLS policy primitives.
//!
//! Run: `cargo run --example tls --features enterprise`

#[cfg(feature = "enterprise")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "enterprise")]
fn main() {
    use http_handle::enterprise::{
        AuthPolicy, TlsPolicy, validate_mtls_subject,
    };
    use std::path::PathBuf;

    support::header("http-handle -- tls");

    support::task_with_output(
        "Default TlsPolicy is disabled until paths are wired",
        || {
            let policy = TlsPolicy::default();
            vec![
                format!("enabled       = {}", policy.enabled),
                format!("mtls_enabled  = {}", policy.mtls_enabled),
                format!("cert_chain    = {:?}", policy.cert_chain_path),
                format!(
                    "private_key   = {:?}",
                    policy.private_key_path
                ),
            ]
        },
    );

    support::task_with_output(
        "Production-shaped TLS + mTLS policy",
        || {
            let policy = TlsPolicy {
                enabled: true,
                cert_chain_path: Some(PathBuf::from(
                    "/etc/ssl/server.crt",
                )),
                private_key_path: Some(PathBuf::from(
                    "/etc/ssl/server.key",
                )),
                mtls_enabled: true,
                client_ca_bundle_path: Some(PathBuf::from(
                    "/etc/ssl/clients.bundle",
                )),
            };
            vec![
                format!("enabled       = {}", policy.enabled),
                format!("mtls_enabled  = {}", policy.mtls_enabled),
                format!(
                    "client_ca     = {}",
                    policy
                        .client_ca_bundle_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                ),
            ]
        },
    );

    support::task_with_output(
        "validate_mtls_subject filters against an allowlist",
        || {
            let auth = AuthPolicy {
                mtls_subject_allowlist: vec![
                    "CN=svc-a,O=acme".into(),
                    "CN=svc-b,O=acme".into(),
                ],
                ..AuthPolicy::default()
            };
            let allowed =
                validate_mtls_subject(&auth, "CN=svc-a,O=acme");
            let denied =
                validate_mtls_subject(&auth, "CN=stranger,O=evil");
            vec![
                format!("CN=svc-a,O=acme    -> {allowed}"),
                format!("CN=stranger,O=evil -> {denied}"),
            ]
        },
    );

    support::summary(3);
}

#[cfg(not(feature = "enterprise"))]
fn main() {
    eprintln!(
        "Enable the 'enterprise' feature: cargo run --example tls --features enterprise"
    );
}
