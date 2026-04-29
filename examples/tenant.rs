// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `multi-tenant` feature: per-tenant config + secret isolation.
//!
//! Run: `cargo run --example tenant --features multi-tenant`

#[cfg(feature = "multi-tenant")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "multi-tenant")]
fn main() {
    use http_handle::tenant_isolation::{
        StaticSecretProvider, TenantConfig, TenantConfigStore,
        TenantId, TenantScopedSecrets,
    };

    support::header("http-handle -- tenant");

    let store = TenantConfigStore::default();
    let acme = TenantId("acme".into());
    let globex = TenantId("globex".into());

    support::task_with_output(
        "Per-tenant TenantConfig is namespaced by TenantId",
        || {
            store
                .set_config(
                    acme.clone(),
                    TenantConfig {
                        settings: [("mode".into(), "prod".into())]
                            .into_iter()
                            .collect(),
                    },
                )
                .expect("acme config");
            store
                .set_config(
                    globex.clone(),
                    TenantConfig {
                        settings: [("mode".into(), "staging".into())]
                            .into_iter()
                            .collect(),
                    },
                )
                .expect("globex config");
            let acme_mode = store
                .get_config(&acme)
                .ok()
                .flatten()
                .and_then(|cfg| cfg.settings.get("mode").cloned())
                .unwrap_or_default();
            let globex_mode = store
                .get_config(&globex)
                .ok()
                .flatten()
                .and_then(|cfg| cfg.settings.get("mode").cloned())
                .unwrap_or_default();
            vec![
                format!("acme.mode   = {acme_mode}"),
                format!("globex.mode = {globex_mode}"),
            ]
        },
    );

    support::task_with_output(
        "Secrets are scoped to the tenant that owns them",
        || {
            let provider = StaticSecretProvider::default()
                .with_secret(acme.clone(), "db_password", "acme-pw")
                .with_secret(
                    globex.clone(),
                    "db_password",
                    "globex-pw",
                );
            let secrets = TenantScopedSecrets::new(provider);
            let acme_secret = secrets
                .read(&acme, "db_password")
                .expect("read")
                .expect("present");
            let globex_secret = secrets
                .read(&globex, "db_password")
                .expect("read")
                .expect("present");
            let cross =
                secrets.read(&acme, "missing-key").expect("read");
            vec![
                format!(
                    "acme.db_password.len   = {}",
                    acme_secret.len()
                ),
                format!(
                    "globex.db_password.len = {}",
                    globex_secret.len()
                ),
                format!("acme.missing-key       = {:?}", cross),
            ]
        },
    );

    support::summary(2);
}

#[cfg(not(feature = "multi-tenant"))]
fn main() {
    eprintln!(
        "Enable the 'multi-tenant' feature: cargo run --example tenant --features multi-tenant"
    );
}
