// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `auth` (via `enterprise` umbrella): API key + JWT verifiers.
//!
//! Run: `cargo run --example auth --features enterprise`

#[cfg(feature = "enterprise")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "enterprise")]
fn main() {
    use http_handle::enterprise::{
        AuthPolicy, validate_api_key, validate_jwt,
    };

    support::header("http-handle -- auth");

    support::task_with_output("Static API key allowlist", || {
        let policy = AuthPolicy {
            api_keys: vec!["k-prod".into(), "k-staging".into()],
            ..AuthPolicy::default()
        };
        vec![
            format!(
                "k-prod    -> {}",
                validate_api_key(&policy, "k-prod")
            ),
            format!(
                "k-staging -> {}",
                validate_api_key(&policy, "k-staging")
            ),
            format!(
                "k-rogue   -> {}",
                validate_api_key(&policy, "k-rogue")
            ),
        ]
    });

    support::task_with_output(
        "validate_jwt: shape-only check by default (3 dot-separated segments)",
        || {
            let policy = AuthPolicy::default();
            let well_formed =
                validate_jwt(&policy, "header.payload.signature");
            let too_few = validate_jwt(&policy, "only.two");
            vec![
                format!(
                    "header.payload.signature  -> {:?}",
                    well_formed.map(|_| "ok")
                ),
                format!(
                    "only.two                  -> err? {}",
                    too_few.is_err()
                ),
            ]
        },
    );

    support::task_with_output(
        "JWT secret env var must exist when configured",
        || {
            let policy = AuthPolicy {
                jwt_secret_env: Some(
                    "HTTP_HANDLE_JWT_SECRET_NOT_SET".into(),
                ),
                ..AuthPolicy::default()
            };
            let err = validate_jwt(&policy, "a.b.c").err();
            vec![format!(
                "missing env  -> err? {} ({})",
                err.is_some(),
                err.as_ref().map(|e| e.to_string()).unwrap_or_default()
            )]
        },
    );

    support::summary(3);
}

#[cfg(not(feature = "enterprise"))]
fn main() {
    eprintln!(
        "Enable the 'enterprise' feature: cargo run --example auth --features enterprise"
    );
}
