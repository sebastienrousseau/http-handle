// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `config` (via `enterprise` umbrella): TOML profiles + hot reload.
//!
//! Run: `cargo run --example config --features enterprise`

#[cfg(feature = "enterprise")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "enterprise")]
fn main() {
    use http_handle::enterprise::{
        EnterpriseConfig, EnterpriseConfigReloader,
    };

    support::header("http-handle -- config");

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("enterprise.toml");

    support::task_with_output(
        "Production baseline serialises and round-trips through TOML",
        || {
            let baseline = EnterpriseConfig::production_baseline();
            baseline.save_to_file(&path).expect("save");
            let loaded =
                EnterpriseConfig::load_from_file(&path).expect("load");
            assert_eq!(baseline, loaded);
            vec![
                format!("path     = {}", path.display()),
                format!("tls.on   = {}", loaded.tls.enabled),
                format!("mtls.on  = {}", loaded.tls.mtls_enabled),
                format!(
                    "otlp     = {} -> {}",
                    loaded.telemetry.otlp_enabled,
                    loaded
                        .telemetry
                        .otlp_endpoint
                        .as_deref()
                        .unwrap_or("(none)")
                ),
            ]
        },
    );

    support::task_with_output(
        "EnterpriseConfigReloader watches the file for changes",
        || {
            let reloader =
                EnterpriseConfigReloader::watch(&path).expect("watch");
            let snapshot = reloader.snapshot();
            vec![
                format!(
                    "snapshot.profile = {:?}",
                    snapshot.profile
                ),
                "Mutations to the watched file replace the snapshot atomically.".into(),
                "Read-side: snapshot() returns Arc<EnterpriseConfig>; no locks on the hot path.".into(),
            ]
        },
    );

    support::summary(2);
}

#[cfg(not(feature = "enterprise"))]
fn main() {
    eprintln!(
        "Enable the 'enterprise' feature: cargo run --example config --features enterprise"
    );
}
