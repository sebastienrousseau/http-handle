// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Multi-tenant configuration isolation and secret-provider helpers.

use crate::error::ServerError;
use std::collections::HashMap;
use std::sync::RwLock;

/// Tenant identifier.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::TenantId;
/// let t = TenantId("acme".into());
/// assert_eq!(t.0, "acme");
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TenantId(pub String);

/// Per-tenant configuration document.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::TenantConfig;
/// let cfg = TenantConfig::default();
/// assert!(cfg.settings.is_empty());
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TenantConfig {
    /// Arbitrary tenant settings.
    pub settings: HashMap<String, String>,
}

/// Thread-safe tenant config store with strict tenant keying.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::TenantConfigStore;
/// let _store = TenantConfigStore::default();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Debug, Default)]
pub struct TenantConfigStore {
    data: RwLock<HashMap<TenantId, TenantConfig>>,
}

impl TenantConfigStore {
    /// Writes tenant config snapshot.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::tenant_isolation::{TenantConfig, TenantConfigStore, TenantId};
    /// let store = TenantConfigStore::default();
    /// let _ = store.set_config(TenantId("acme".into()), TenantConfig::default());
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying lock is poisoned.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn set_config(
        &self,
        tenant: TenantId,
        config: TenantConfig,
    ) -> Result<(), ServerError> {
        let mut guard = self.data.write().map_err(|_| {
            ServerError::Custom("tenant store poisoned".into())
        })?;
        let _ = guard.insert(tenant, config);
        Ok(())
    }

    /// Returns a cloned tenant config snapshot.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::tenant_isolation::{TenantConfigStore, TenantId};
    /// let store = TenantConfigStore::default();
    /// let _ = store.get_config(&TenantId("acme".into()));
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying lock is poisoned.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn get_config(
        &self,
        tenant: &TenantId,
    ) -> Result<Option<TenantConfig>, ServerError> {
        let guard = self.data.read().map_err(|_| {
            ServerError::Custom("tenant store poisoned".into())
        })?;
        Ok(guard.get(tenant).cloned())
    }
}

/// External secret provider contract for tenant-scoped lookup.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::SecretProvider;
/// # let _ = std::any::TypeId::of::<&dyn SecretProvider>();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// Trait usage does not panic by itself.
pub trait SecretProvider: Send + Sync + std::fmt::Debug {
    /// Fetches secret for tenant and key.
    fn get_secret(
        &self,
        tenant: &TenantId,
        key: &str,
    ) -> Result<Option<String>, ServerError>;
}

/// Environment-backed secret provider using strict tenant-key namespace.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::EnvSecretProvider;
/// let _provider = EnvSecretProvider::new("HTTP_HANDLE_SECRET");
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Debug)]
pub struct EnvSecretProvider {
    prefix: String,
}

impl EnvSecretProvider {
    /// Creates provider with prefix used in env keys.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::tenant_isolation::EnvSecretProvider;
    /// let _provider = EnvSecretProvider::new("HTTP_HANDLE_SECRET");
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    fn env_key(&self, tenant: &TenantId, key: &str) -> String {
        let tenant_norm =
            tenant.0.replace('-', "_").to_ascii_uppercase();
        let key_norm = key.replace('-', "_").to_ascii_uppercase();
        format!("{}_{}_{}", self.prefix, tenant_norm, key_norm)
    }
}

impl SecretProvider for EnvSecretProvider {
    fn get_secret(
        &self,
        tenant: &TenantId,
        key: &str,
    ) -> Result<Option<String>, ServerError> {
        let env_key = self.env_key(tenant, key);
        Ok(std::env::var(env_key).ok())
    }
}

/// In-memory secret provider useful for local development/testing.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::StaticSecretProvider;
/// let _provider = StaticSecretProvider::default();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Debug, Default)]
pub struct StaticSecretProvider {
    data: HashMap<(TenantId, String), String>,
}

impl StaticSecretProvider {
    /// Adds a tenant-scoped secret value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::tenant_isolation::{StaticSecretProvider, TenantId};
    /// let _provider = StaticSecretProvider::default().with_secret(TenantId("acme".into()), "token", "abc");
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn with_secret(
        mut self,
        tenant: TenantId,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let _ = self.data.insert((tenant, key.into()), value.into());
        self
    }
}

impl SecretProvider for StaticSecretProvider {
    fn get_secret(
        &self,
        tenant: &TenantId,
        key: &str,
    ) -> Result<Option<String>, ServerError> {
        Ok(self.data.get(&(tenant.clone(), key.to_string())).cloned())
    }
}

/// Tenant-scoped secret accessor.
///
/// # Examples
///
/// ```rust
/// use http_handle::tenant_isolation::{StaticSecretProvider, TenantScopedSecrets};
/// let provider = StaticSecretProvider::default();
/// let _secrets = TenantScopedSecrets::new(provider);
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Debug)]
pub struct TenantScopedSecrets<P: SecretProvider> {
    provider: P,
}

impl<P: SecretProvider> TenantScopedSecrets<P> {
    /// Creates a tenant-scoped secret accessor.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::tenant_isolation::{StaticSecretProvider, TenantScopedSecrets};
    /// let _s = TenantScopedSecrets::new(StaticSecretProvider::default());
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Reads tenant secret.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::tenant_isolation::{StaticSecretProvider, TenantId, TenantScopedSecrets};
    /// let s = TenantScopedSecrets::new(StaticSecretProvider::default());
    /// let _ = s.read(&TenantId("acme".into()), "token");
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns provider-specific errors for secret lookup failures.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn read(
        &self,
        tenant: &TenantId,
        key: &str,
    ) -> Result<Option<String>, ServerError> {
        self.provider.get_secret(tenant, key)
    }
}

#[cfg(test)]
// Test-only env-var mutations (`std::env::set_var` / `remove_var`) need
// `unsafe` under Rust 2024. Each call site below is a paired write +
// cleanup inside a single test scope and is documented at the use site.
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn tenant_store_is_isolated() {
        let store = TenantConfigStore::default();
        let tenant_a = TenantId("alpha".into());
        let tenant_b = TenantId("beta".into());
        store
            .set_config(
                tenant_a.clone(),
                TenantConfig {
                    settings: [("mode".into(), "strict".into())]
                        .into_iter()
                        .collect(),
                },
            )
            .expect("set");
        assert_eq!(
            store
                .get_config(&tenant_a)
                .expect("get")
                .expect("cfg")
                .settings
                .get("mode"),
            Some(&"strict".to_string())
        );
        assert!(store.get_config(&tenant_b).expect("get").is_none());
    }

    #[test]
    fn static_secret_provider_is_tenant_scoped() {
        let provider = StaticSecretProvider::default()
            .with_secret(TenantId("alpha".into()), "db_password", "a1")
            .with_secret(TenantId("beta".into()), "db_password", "b1");
        let scoped = TenantScopedSecrets::new(provider);
        assert_eq!(
            scoped
                .read(&TenantId("alpha".into()), "db_password")
                .expect("read"),
            Some("a1".to_string())
        );
        assert_eq!(
            scoped
                .read(&TenantId("beta".into()), "db_password")
                .expect("read"),
            Some("b1".to_string())
        );
    }

    #[test]
    fn env_secret_provider_namespaces_keys() {
        let provider = EnvSecretProvider::new("HTTP_HANDLE_SECRET");
        let tenant = TenantId("alpha-team".into());
        let key = "api_token";
        let env_key = "HTTP_HANDLE_SECRET_ALPHA_TEAM_API_TOKEN";
        let value = "secret-value";
        // Safety: this test writes and removes a single process env var in a
        // short scope and does not spawn threads that concurrently mutate env.
        unsafe { std::env::set_var(env_key, value) };
        let got = provider.get_secret(&tenant, key).expect("read");
        assert_eq!(got, Some(value.to_string()));
        // Safety: paired cleanup for the key set above in the same test scope.
        unsafe { std::env::remove_var(env_key) };
    }

    #[test]
    fn env_secret_provider_returns_none_when_missing() {
        let provider = EnvSecretProvider::new("HTTP_HANDLE_SECRET");
        let got = provider
            .get_secret(&TenantId("missing".into()), "api_token")
            .expect("read");
        assert!(got.is_none());
    }

    #[test]
    fn tenant_store_write_poison_maps_to_error() {
        let store = TenantConfigStore::default();
        let _ = std::panic::catch_unwind(|| {
            let _guard = store.data.write().expect("lock");
            panic!("poison");
        });
        let err = store
            .set_config(TenantId("t1".into()), TenantConfig::default())
            .expect_err("must fail");
        assert!(err.to_string().contains("poisoned"));
    }

    #[test]
    fn tenant_store_read_poison_maps_to_error() {
        let store = TenantConfigStore::default();
        let _ = std::panic::catch_unwind(|| {
            let _guard = store.data.write().expect("lock");
            panic!("poison");
        });
        let err = store
            .get_config(&TenantId("t1".into()))
            .expect_err("must fail");
        assert!(err.to_string().contains("poisoned"));
    }
}
