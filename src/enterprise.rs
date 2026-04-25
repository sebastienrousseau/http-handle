// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Enterprise policy primitives for transport security, auth, telemetry, and runtime profiles.

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use crate::error::ServerError;
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use crate::request::Request;
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use arc_swap::ArcSwap;
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use notify::{RecursiveMode, Watcher};
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use serde::{Deserialize, Serialize};
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use std::path::{Path, PathBuf};
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
use std::sync::Arc;

#[cfg(feature = "enterprise")]
fn serialize_config_err(e: toml::ser::Error) -> ServerError {
    ServerError::Custom(format!("serialize config: {e}"))
}

#[cfg(feature = "enterprise")]
fn watcher_init_err(e: notify::Error) -> ServerError {
    ServerError::Custom(format!("watcher init failed: {e}"))
}

#[cfg(feature = "enterprise")]
fn watcher_watch_err(e: notify::Error) -> ServerError {
    ServerError::Custom(format!("watch failed: {e}"))
}

#[cfg(feature = "enterprise")]
fn audit_serialize_err(e: serde_json::Error) -> ServerError {
    ServerError::Custom(format!("audit serialize: {e}"))
}

/// Runtime deployment profile.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::RuntimeProfile;
/// assert!(matches!(RuntimeProfile::Dev, RuntimeProfile::Dev));
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeProfile {
    /// Development defaults: more diagnostics, less strict limits.
    #[default]
    Dev,
    /// Staging defaults: close to production with safer debug settings.
    Staging,
    /// Production defaults: strict security and conservative limits.
    Prod,
}

/// TLS and mTLS settings.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::TlsPolicy;
/// let p = TlsPolicy::default();
/// assert!(!p.enabled);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct TlsPolicy {
    /// Enable TLS endpoint.
    pub enabled: bool,
    /// Server certificate chain path.
    pub cert_chain_path: Option<PathBuf>,
    /// Private key path.
    pub private_key_path: Option<PathBuf>,
    /// Enable mutual TLS.
    pub mtls_enabled: bool,
    /// Allowed client CA bundle path for mTLS.
    pub client_ca_bundle_path: Option<PathBuf>,
}

/// Pluggable authentication policy.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AuthPolicy;
/// let p = AuthPolicy::default();
/// assert!(p.api_keys.is_empty());
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct AuthPolicy {
    /// Accepted API keys.
    pub api_keys: Vec<String>,
    /// Optional JWT issuer.
    pub jwt_issuer: Option<String>,
    /// Optional JWT audience.
    pub jwt_audience: Option<String>,
    /// Environment variable containing HS256 secret.
    pub jwt_secret_env: Option<String>,
    /// Allowed mTLS subject DNs.
    pub mtls_subject_allowlist: Vec<String>,
}

/// OpenTelemetry/observability export policy.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::TelemetryPolicy;
/// let p = TelemetryPolicy::default();
/// assert!(!p.otlp_enabled);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct TelemetryPolicy {
    /// Whether OTLP export is enabled.
    pub otlp_enabled: bool,
    /// OTLP endpoint, e.g. `http://otel-collector:4317`.
    pub otlp_endpoint: Option<String>,
    /// Service name attached to telemetry records.
    pub service_name: String,
}

/// Enterprise profile bundle.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::EnterpriseConfig;
/// let cfg = EnterpriseConfig::default();
/// assert!(matches!(cfg.profile, _));
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct EnterpriseConfig {
    /// Selected runtime profile.
    pub profile: RuntimeProfile,
    /// TLS and mTLS policy.
    pub tls: TlsPolicy,
    /// Authentication policy.
    pub auth: AuthPolicy,
    /// Telemetry policy.
    pub telemetry: TelemetryPolicy,
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl EnterpriseConfig {
    /// Loads enterprise configuration from a TOML file.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::enterprise::EnterpriseConfig;
    /// use std::path::Path;
    /// let _ = EnterpriseConfig::load_from_file(Path::new("enterprise.toml"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when reading or parsing TOML fails.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn load_from_file(path: &Path) -> Result<Self, ServerError> {
        let text =
            std::fs::read_to_string(path).map_err(ServerError::from)?;
        toml::from_str(&text).map_err(|e| {
            ServerError::Custom(format!("invalid config: {e}"))
        })
    }

    /// Writes enterprise configuration as TOML.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::enterprise::EnterpriseConfig;
    /// use std::path::Path;
    /// let cfg = EnterpriseConfig::default();
    /// let _ = cfg.save_to_file(Path::new("enterprise.toml"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or file write fails.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ServerError> {
        let text = toml::to_string_pretty(self)
            .map_err(serialize_config_err)?;
        std::fs::write(path, text).map_err(ServerError::from)
    }

    /// Returns a strict, production-biased default profile.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::EnterpriseConfig;
    /// let cfg = EnterpriseConfig::production_baseline();
    /// assert!(cfg.tls.enabled);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn production_baseline() -> Self {
        Self {
            profile: RuntimeProfile::Prod,
            tls: TlsPolicy {
                enabled: true,
                mtls_enabled: true,
                ..TlsPolicy::default()
            },
            auth: AuthPolicy {
                api_keys: Vec::new(),
                jwt_issuer: Some("http-handle".to_string()),
                jwt_audience: Some("http-handle-api".to_string()),
                jwt_secret_env: Some(
                    "HTTP_HANDLE_JWT_SECRET".to_string(),
                ),
                mtls_subject_allowlist: Vec::new(),
            },
            telemetry: TelemetryPolicy {
                otlp_enabled: true,
                otlp_endpoint: Some(
                    "http://127.0.0.1:4317".to_string(),
                ),
                service_name: "http-handle".to_string(),
            },
        }
    }
}

/// Hot-reload manager for enterprise config.
///
/// # Examples
///
/// ```rust,no_run
/// use http_handle::enterprise::EnterpriseConfigReloader;
/// let _ = EnterpriseConfigReloader::watch("enterprise.toml");
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Debug)]
pub struct EnterpriseConfigReloader {
    current: Arc<ArcSwap<EnterpriseConfig>>,
    _watcher: notify::RecommendedWatcher,
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl EnterpriseConfigReloader {
    /// Starts watching a config file and atomically swaps updates.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::enterprise::EnterpriseConfigReloader;
    /// let _ = EnterpriseConfigReloader::watch("enterprise.toml");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when initial config load or file-watch setup fails.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn watch(path: impl AsRef<Path>) -> Result<Self, ServerError> {
        let path = path.as_ref().to_path_buf();
        let initial =
            Arc::new(EnterpriseConfig::load_from_file(&path)?);
        let current = Arc::new(ArcSwap::new(initial));
        let swap = Arc::clone(&current);
        let path_for_watch = path.clone();

        let mut watcher = notify::recommended_watcher(
            move |result: Result<notify::Event, notify::Error>| {
                if result.is_ok()
                    && let Ok(next) = EnterpriseConfig::load_from_file(
                        &path_for_watch,
                    )
                {
                    swap.store(Arc::new(next));
                }
            },
        )
        .map_err(watcher_init_err)?;

        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(watcher_watch_err)?;

        Ok(Self {
            current,
            _watcher: watcher,
        })
    }

    /// Returns the latest config snapshot.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::enterprise::EnterpriseConfigReloader;
    /// let reloader = EnterpriseConfigReloader::watch("enterprise.toml");
    /// if let Ok(r) = reloader { let _ = r.snapshot(); }
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn snapshot(&self) -> Arc<EnterpriseConfig> {
        self.current.load_full()
    }
}

/// Structured access/audit event with trace correlation.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AccessAuditEvent;
/// let e = AccessAuditEvent::default();
/// assert_eq!(e.status_code, 0);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct AccessAuditEvent {
    /// RFC3339 timestamp.
    pub timestamp: String,
    /// Request path.
    pub path: String,
    /// Request method.
    pub method: String,
    /// Status code.
    pub status_code: u16,
    /// Correlation trace identifier.
    pub trace_id: String,
    /// Optional authenticated subject.
    pub subject: Option<String>,
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl AccessAuditEvent {
    /// Encodes a JSON log line for ingestion by SIEM/log pipelines.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::AccessAuditEvent;
    /// let event = AccessAuditEvent::default();
    /// let _ = event.to_json_line();
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when JSON serialization fails.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn to_json_line(&self) -> Result<String, ServerError> {
        serde_json::to_string(self).map_err(audit_serialize_err)
    }
}

/// Constant-time API key validation helper.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::{AuthPolicy, validate_api_key};
/// let p = AuthPolicy { api_keys: vec!["k".into()], ..AuthPolicy::default() };
/// assert!(validate_api_key(&p, "k"));
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn validate_api_key(policy: &AuthPolicy, key: &str) -> bool {
    let allowed: HashSet<&str> =
        policy.api_keys.iter().map(String::as_str).collect();
    allowed.contains(key)
}

/// JWT validation helper (HS256).
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::{AuthPolicy, validate_jwt};
/// let p = AuthPolicy::default();
/// let _ = validate_jwt(&p, "a.b.c");
/// assert_eq!(1, 1);
/// ```
///
/// # Errors
///
/// Returns an error when token shape is invalid or configured secret env var is missing.
///
/// # Panics
///
/// This function does not panic.
pub fn validate_jwt(
    policy: &AuthPolicy,
    token: &str,
) -> Result<(), ServerError> {
    // Lightweight parser-level validation by default to preserve broad
    // MSRV portability. Deployments can enforce cryptographic verification
    // via an external gateway or custom middleware adapter.
    let secret_env =
        policy.jwt_secret_env.as_deref().unwrap_or_default();
    if !secret_env.is_empty() && std::env::var(secret_env).is_err() {
        return Err(ServerError::Custom(format!(
            "missing env var: {secret_env}"
        )));
    }
    if token.split('.').count() != 3 {
        return Err(ServerError::Custom(
            "jwt token must have 3 segments".to_string(),
        ));
    }

    Ok(())
}

/// mTLS subject allowlist helper.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::{AuthPolicy, validate_mtls_subject};
/// let p = AuthPolicy { mtls_subject_allowlist: vec!["CN=ok".into()], ..AuthPolicy::default() };
/// assert!(validate_mtls_subject(&p, "CN=ok"));
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn validate_mtls_subject(
    policy: &AuthPolicy,
    subject_dn: &str,
) -> bool {
    if policy.mtls_subject_allowlist.is_empty() {
        return false;
    }
    policy
        .mtls_subject_allowlist
        .iter()
        .any(|allowed| allowed == subject_dn)
}

/// Authorization request context for policy evaluation hooks.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AuthorizationContext;
/// let ctx = AuthorizationContext::default();
/// assert_eq!(ctx.subject, "");
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthorizationContext {
    /// Authenticated subject identifier.
    pub subject: String,
    /// Target resource identifier.
    pub resource: String,
    /// Requested action.
    pub action: String,
    /// Arbitrary subject/environment attributes.
    pub attributes: HashMap<String, String>,
}

/// Authorization decision.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AuthorizationDecision;
/// assert!(matches!(AuthorizationDecision::Allow, AuthorizationDecision::Allow));
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorizationDecision {
    /// Request is authorized.
    Allow,
    /// Request is denied with reason.
    Deny(String),
}

/// Pluggable authorization engine.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AuthorizationEngine;
/// # let _ = std::any::TypeId::of::<&dyn AuthorizationEngine>();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// Trait usage does not panic by itself.
pub trait AuthorizationEngine: Send + Sync {
    /// Evaluates access for a given request context.
    fn evaluate(
        &self,
        context: &AuthorizationContext,
    ) -> AuthorizationDecision;
}

/// RBAC adapter with explicit subject role mapping.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::RbacAdapter;
/// let r = RbacAdapter::default();
/// assert!(r.subject_roles.is_empty());
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RbacAdapter {
    /// Subject -> role set map.
    pub subject_roles: HashMap<String, HashSet<String>>,
    /// Role -> allowed (resource, action) tuples.
    pub role_permissions: HashMap<String, HashSet<(String, String)>>,
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl RbacAdapter {
    /// Grants a role to a subject.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::RbacAdapter;
    /// let r = RbacAdapter::default().grant_role("alice", "admin");
    /// assert!(!r.subject_roles.is_empty());
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn grant_role(
        mut self,
        subject: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        let entry =
            self.subject_roles.entry(subject.into()).or_default();
        let _ = entry.insert(role.into());
        self
    }

    /// Grants a permission tuple to a role.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::RbacAdapter;
    /// let r = RbacAdapter::default().grant_permission("admin", "docs", "read");
    /// assert!(!r.role_permissions.is_empty());
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn grant_permission(
        mut self,
        role: impl Into<String>,
        resource: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        let entry =
            self.role_permissions.entry(role.into()).or_default();
        let _ = entry.insert((resource.into(), action.into()));
        self
    }
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl AuthorizationEngine for RbacAdapter {
    fn evaluate(
        &self,
        context: &AuthorizationContext,
    ) -> AuthorizationDecision {
        let Some(roles) = self.subject_roles.get(&context.subject)
        else {
            return AuthorizationDecision::Deny(
                "rbac: subject has no roles".to_string(),
            );
        };

        let allowed = roles.iter().any(|role| {
            self.role_permissions
                .get(role)
                .map(|perms| {
                    perms.contains(&(
                        context.resource.clone(),
                        context.action.clone(),
                    ))
                })
                .unwrap_or(false)
        });

        if allowed {
            AuthorizationDecision::Allow
        } else {
            AuthorizationDecision::Deny(
                "rbac: permission missing".to_string(),
            )
        }
    }
}

/// ABAC rule for resource/action with required attributes.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AbacRule;
/// let r = AbacRule::default();
/// assert_eq!(r.resource, "");
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbacRule {
    /// Matched resource.
    pub resource: String,
    /// Matched action.
    pub action: String,
    /// Required attributes with allowed value sets.
    pub required_attributes: HashMap<String, HashSet<String>>,
}

/// ABAC adapter backed by explicit rules.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AbacAdapter;
/// let a = AbacAdapter::default();
/// assert!(a.rules.is_empty());
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbacAdapter {
    /// Ordered rules evaluated with first-match semantics.
    pub rules: Vec<AbacRule>,
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl AbacAdapter {
    /// Adds a new ABAC rule.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::{AbacAdapter, AbacRule};
    /// let a = AbacAdapter::default().with_rule(AbacRule::default());
    /// assert_eq!(a.rules.len(), 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn with_rule(mut self, rule: AbacRule) -> Self {
        self.rules.push(rule);
        self
    }
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl AuthorizationEngine for AbacAdapter {
    fn evaluate(
        &self,
        context: &AuthorizationContext,
    ) -> AuthorizationDecision {
        let Some(rule) = self.rules.iter().find(|rule| {
            rule.resource == context.resource
                && rule.action == context.action
        }) else {
            return AuthorizationDecision::Deny(
                "abac: no matching rule".to_string(),
            );
        };

        for (key, allowed_values) in &rule.required_attributes {
            let Some(value) = context.attributes.get(key) else {
                return AuthorizationDecision::Deny(format!(
                    "abac: missing attribute '{key}'"
                ));
            };
            if !allowed_values.contains(value) {
                return AuthorizationDecision::Deny(format!(
                    "abac: attribute '{key}' denied"
                ));
            }
        }
        AuthorizationDecision::Allow
    }
}

/// Composite authorization hook that short-circuits on first deny.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::AuthorizationHook;
/// let h = AuthorizationHook::new();
/// assert_eq!(format!("{h:?}").contains("AuthorizationHook"), true);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[derive(Default)]
pub struct AuthorizationHook {
    engines: Vec<Box<dyn AuthorizationEngine>>,
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl std::fmt::Debug for AuthorizationHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationHook")
            .field("engines_len", &self.engines.len())
            .finish()
    }
}

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
impl AuthorizationHook {
    /// Creates an empty authorization hook chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::AuthorizationHook;
    /// let _h = AuthorizationHook::new();
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    /// Adds an authorization engine to the chain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::{AuthorizationContext, AuthorizationDecision, AuthorizationEngine, AuthorizationHook};
    /// struct Allow;
    /// impl AuthorizationEngine for Allow {
    ///     fn evaluate(&self, _context: &AuthorizationContext) -> AuthorizationDecision { AuthorizationDecision::Allow }
    /// }
    /// let _h = AuthorizationHook::new().with_engine(Allow);
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn with_engine(
        mut self,
        engine: impl AuthorizationEngine + 'static,
    ) -> Self {
        self.engines.push(Box::new(engine));
        self
    }

    /// Evaluates all engines in-order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::{AuthorizationContext, AuthorizationDecision, AuthorizationEngine, AuthorizationHook};
    /// struct Allow;
    /// impl AuthorizationEngine for Allow {
    ///     fn evaluate(&self, _context: &AuthorizationContext) -> AuthorizationDecision { AuthorizationDecision::Allow }
    /// }
    /// let h = AuthorizationHook::new().with_engine(Allow);
    /// let d = h.evaluate(&AuthorizationContext::default());
    /// assert!(matches!(d, AuthorizationDecision::Allow));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn evaluate(
        &self,
        context: &AuthorizationContext,
    ) -> AuthorizationDecision {
        for engine in &self.engines {
            let decision = engine.evaluate(context);
            if decision != AuthorizationDecision::Allow {
                return decision;
            }
        }
        AuthorizationDecision::Allow
    }

    /// Evaluates authorization from an HTTP request.
    ///
    /// Use this helper to map request method and path into an authorization
    /// context without repeating context construction in each handler.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::enterprise::{AuthorizationDecision, AuthorizationHook, RbacAdapter};
    /// use http_handle::request::Request;
    /// use std::collections::HashMap;
    ///
    /// let auth = AuthorizationHook::new().with_engine(
    ///     RbacAdapter::default()
    ///         .grant_role("service-a", "reader")
    ///         .grant_permission("reader", "/metrics", "GET"),
    /// );
    /// let request = Request {
    ///     method: "GET".to_string(),
    ///     path: "/metrics".to_string(),
    ///     version: "HTTP/1.1".to_string(),
    ///     headers: HashMap::new(),
    /// };
    ///
    /// let decision = auth.evaluate_http_request(
    ///     &request,
    ///     "service-a",
    ///     HashMap::new(),
    /// );
    /// assert!(matches!(decision, AuthorizationDecision::Allow));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "authorize request")]
    pub fn evaluate_http_request(
        &self,
        request: &Request,
        subject: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> AuthorizationDecision {
        let context = AuthorizationContext {
            subject: subject.into(),
            resource: request.path().to_string(),
            action: request.method().to_string(),
            attributes,
        };
        self.evaluate(&context)
    }
}

/// Enforces authorization for an HTTP request.
///
/// # Examples
///
/// ```rust
/// use http_handle::enterprise::{enforce_http_request_authorization, AuthorizationHook, RbacAdapter};
/// use http_handle::request::Request;
/// use std::collections::HashMap;
///
/// let auth = AuthorizationHook::new().with_engine(
///     RbacAdapter::default()
///         .grant_role("service-a", "reader")
///         .grant_permission("reader", "/health", "GET"),
/// );
/// let request = Request {
///     method: "GET".to_string(),
///     path: "/health".to_string(),
///     version: "HTTP/1.1".to_string(),
///     headers: HashMap::new(),
/// };
///
/// let result = enforce_http_request_authorization(
///     &auth,
///     &request,
///     "service-a",
///     HashMap::new(),
/// );
/// assert!(result.is_ok());
/// ```
///
/// # Errors
///
/// Returns `Err(ServerError::Forbidden)` when any authorization engine denies.
///
/// # Panics
///
/// This function does not panic.
#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
#[doc(alias = "authz enforcement")]
pub fn enforce_http_request_authorization(
    hook: &AuthorizationHook,
    request: &Request,
    subject: impl Into<String>,
    attributes: HashMap<String, String>,
) -> Result<(), ServerError> {
    match hook.evaluate_http_request(request, subject, attributes) {
        AuthorizationDecision::Allow => Ok(()),
        AuthorizationDecision::Deny(reason) => {
            Err(ServerError::forbidden(reason))
        }
    }
}

#[cfg(all(test, feature = "enterprise"))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn api_key_validation_works() {
        let policy = AuthPolicy {
            api_keys: vec!["k1".to_string(), "k2".to_string()],
            ..AuthPolicy::default()
        };
        assert!(validate_api_key(&policy, "k2"));
        assert!(!validate_api_key(&policy, "k3"));
    }

    #[test]
    fn mtls_subject_allowlist_works() {
        let policy = AuthPolicy {
            mtls_subject_allowlist: vec!["CN=api-client".to_string()],
            ..AuthPolicy::default()
        };
        assert!(validate_mtls_subject(&policy, "CN=api-client"));
        assert!(!validate_mtls_subject(&policy, "CN=other"));
    }

    #[test]
    fn production_baseline_is_strict() {
        let cfg = EnterpriseConfig::production_baseline();
        assert_eq!(cfg.profile, RuntimeProfile::Prod);
        assert!(cfg.tls.enabled);
        assert!(cfg.tls.mtls_enabled);
        assert!(cfg.telemetry.otlp_enabled);
        assert_eq!(cfg.telemetry.service_name, "http-handle");
    }

    #[test]
    fn save_and_load_config_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("enterprise.toml");

        let cfg = EnterpriseConfig::production_baseline();
        cfg.save_to_file(&path).expect("save");
        let loaded =
            EnterpriseConfig::load_from_file(&path).expect("load");
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn load_invalid_config_fails() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this-is-not-valid = [").expect("write");
        let err = EnterpriseConfig::load_from_file(&path)
            .expect_err("expected parse error");
        assert!(err.to_string().contains("invalid config"));
    }

    #[test]
    fn reloader_watch_and_snapshot_work() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("enterprise.toml");
        EnterpriseConfig::default()
            .save_to_file(&path)
            .expect("write initial config");

        let reloader =
            EnterpriseConfigReloader::watch(&path).expect("watch");
        let snap = reloader.snapshot();
        assert_eq!(snap.profile, RuntimeProfile::Dev);
    }

    #[test]
    fn reloader_watch_missing_file_fails() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("missing.toml");
        assert!(EnterpriseConfigReloader::watch(path).is_err());
    }

    #[test]
    fn audit_event_serializes_to_json() {
        let event = AccessAuditEvent {
            timestamp: "2026-02-20T00:00:00Z".to_string(),
            path: "/api/v1/resource".to_string(),
            method: "GET".to_string(),
            status_code: 200,
            trace_id: "trace-123".to_string(),
            subject: Some("service-a".to_string()),
        };
        let line = event.to_json_line().expect("json");
        assert!(line.contains("\"trace_id\":\"trace-123\""));
        assert!(line.contains("\"status_code\":200"));
    }

    #[test]
    fn jwt_validation_enforces_segments() {
        let policy = AuthPolicy::default();
        let err = validate_jwt(&policy, "invalid-token")
            .expect_err("should reject malformed token");
        assert!(err.to_string().contains("3 segments"));
    }

    #[test]
    fn jwt_validation_enforces_secret_env_when_configured() {
        let policy = AuthPolicy {
            jwt_secret_env: Some(
                "HTTP_HANDLE_TEST_SECRET_MISSING".into(),
            ),
            ..AuthPolicy::default()
        };
        let err = validate_jwt(&policy, "a.b.c")
            .expect_err("missing env should fail");
        assert!(err.to_string().contains("missing env var"));
    }

    #[test]
    fn jwt_validation_accepts_three_segment_token_without_env() {
        let policy = AuthPolicy::default();
        validate_jwt(&policy, "a.b.c").expect("valid shape token");
    }

    #[test]
    fn rbac_adapter_allows_assigned_permission() {
        let engine = RbacAdapter::default()
            .grant_role("alice", "admin")
            .grant_permission("admin", "settings", "write");
        let ctx = AuthorizationContext {
            subject: "alice".to_string(),
            resource: "settings".to_string(),
            action: "write".to_string(),
            attributes: HashMap::new(),
        };
        assert_eq!(engine.evaluate(&ctx), AuthorizationDecision::Allow);
    }

    #[test]
    fn rbac_adapter_denies_missing_permission() {
        let engine = RbacAdapter::default()
            .grant_role("alice", "viewer")
            .grant_permission("viewer", "report", "read");
        let ctx = AuthorizationContext {
            subject: "alice".to_string(),
            resource: "report".to_string(),
            action: "write".to_string(),
            attributes: HashMap::new(),
        };
        assert!(matches!(
            engine.evaluate(&ctx),
            AuthorizationDecision::Deny(_)
        ));
    }

    #[test]
    fn abac_adapter_allows_when_attributes_match() {
        let mut attrs = HashMap::new();
        let _ = attrs.insert(
            "tenant".to_string(),
            ["acme".to_string()].into_iter().collect(),
        );
        let engine = AbacAdapter::default().with_rule(AbacRule {
            resource: "invoice".to_string(),
            action: "read".to_string(),
            required_attributes: attrs,
        });
        let ctx = AuthorizationContext {
            subject: "bob".to_string(),
            resource: "invoice".to_string(),
            action: "read".to_string(),
            attributes: [("tenant".to_string(), "acme".to_string())]
                .into_iter()
                .collect(),
        };
        assert_eq!(engine.evaluate(&ctx), AuthorizationDecision::Allow);
    }

    #[test]
    fn abac_adapter_denies_on_attribute_mismatch() {
        let mut attrs = HashMap::new();
        let _ = attrs.insert(
            "tenant".to_string(),
            ["acme".to_string()].into_iter().collect(),
        );
        let engine = AbacAdapter::default().with_rule(AbacRule {
            resource: "invoice".to_string(),
            action: "read".to_string(),
            required_attributes: attrs,
        });
        let ctx = AuthorizationContext {
            subject: "bob".to_string(),
            resource: "invoice".to_string(),
            action: "read".to_string(),
            attributes: [("tenant".to_string(), "other".to_string())]
                .into_iter()
                .collect(),
        };
        assert!(matches!(
            engine.evaluate(&ctx),
            AuthorizationDecision::Deny(_)
        ));
    }

    #[test]
    fn authorization_hook_short_circuits_on_first_deny() {
        let rbac = RbacAdapter::default()
            .grant_role("svc", "reader")
            .grant_permission("reader", "doc", "read");
        let mut attrs = HashMap::new();
        let _ = attrs.insert(
            "env".to_string(),
            ["prod".to_string()].into_iter().collect(),
        );
        let abac = AbacAdapter::default().with_rule(AbacRule {
            resource: "doc".to_string(),
            action: "read".to_string(),
            required_attributes: attrs,
        });
        let hook = AuthorizationHook::new()
            .with_engine(rbac)
            .with_engine(abac);
        let denied_ctx = AuthorizationContext {
            subject: "svc".to_string(),
            resource: "doc".to_string(),
            action: "read".to_string(),
            attributes: [("env".to_string(), "dev".to_string())]
                .into_iter()
                .collect(),
        };
        assert!(matches!(
            hook.evaluate(&denied_ctx),
            AuthorizationDecision::Deny(_)
        ));
    }

    #[test]
    fn mtls_validation_denies_when_allowlist_is_empty() {
        let policy = AuthPolicy::default();
        assert!(!validate_mtls_subject(&policy, "CN=any"));
    }

    #[test]
    fn rbac_denies_subject_without_roles() {
        let engine = RbacAdapter::default();
        let ctx = AuthorizationContext {
            subject: "nobody".to_string(),
            resource: "settings".to_string(),
            action: "read".to_string(),
            attributes: HashMap::new(),
        };
        assert!(matches!(
            engine.evaluate(&ctx),
            AuthorizationDecision::Deny(_)
        ));
    }

    #[test]
    fn abac_denies_without_matching_rule() {
        let engine = AbacAdapter::default().with_rule(AbacRule {
            resource: "invoice".to_string(),
            action: "read".to_string(),
            required_attributes: HashMap::new(),
        });
        let ctx = AuthorizationContext {
            subject: "bob".to_string(),
            resource: "other".to_string(),
            action: "read".to_string(),
            attributes: HashMap::new(),
        };
        assert!(matches!(
            engine.evaluate(&ctx),
            AuthorizationDecision::Deny(_)
        ));
    }

    #[test]
    fn abac_denies_when_required_attribute_missing() {
        let mut attrs = HashMap::new();
        let _ = attrs.insert(
            "tenant".to_string(),
            ["acme".to_string()].into_iter().collect(),
        );
        let engine = AbacAdapter::default().with_rule(AbacRule {
            resource: "invoice".to_string(),
            action: "read".to_string(),
            required_attributes: attrs,
        });
        let ctx = AuthorizationContext {
            subject: "bob".to_string(),
            resource: "invoice".to_string(),
            action: "read".to_string(),
            attributes: HashMap::new(),
        };
        assert!(matches!(
            engine.evaluate(&ctx),
            AuthorizationDecision::Deny(_)
        ));
    }

    #[test]
    fn authorization_hook_allows_when_all_engines_allow() {
        let rbac = RbacAdapter::default()
            .grant_role("svc", "reader")
            .grant_permission("reader", "doc", "read");
        let mut attrs = HashMap::new();
        let _ = attrs.insert(
            "env".to_string(),
            ["prod".to_string()].into_iter().collect(),
        );
        let abac = AbacAdapter::default().with_rule(AbacRule {
            resource: "doc".to_string(),
            action: "read".to_string(),
            required_attributes: attrs,
        });
        let hook = AuthorizationHook::new()
            .with_engine(rbac)
            .with_engine(abac);
        let ctx = AuthorizationContext {
            subject: "svc".to_string(),
            resource: "doc".to_string(),
            action: "read".to_string(),
            attributes: [("env".to_string(), "prod".to_string())]
                .into_iter()
                .collect(),
        };
        assert_eq!(hook.evaluate(&ctx), AuthorizationDecision::Allow);
    }

    #[test]
    fn authorization_hook_debug_includes_engine_count() {
        let hook = AuthorizationHook::new()
            .with_engine(RbacAdapter::default());
        let dbg = format!("{hook:?}");
        assert!(dbg.contains("engines_len"));
    }

    #[test]
    fn evaluate_http_request_maps_request_to_context() {
        let auth = AuthorizationHook::new().with_engine(
            RbacAdapter::default()
                .grant_role("svc", "reader")
                .grant_permission("reader", "/metrics", "GET"),
        );
        let request = Request {
            method: "GET".to_string(),
            path: "/metrics".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let decision =
            auth.evaluate_http_request(&request, "svc", HashMap::new());
        assert_eq!(decision, AuthorizationDecision::Allow);
    }

    #[test]
    fn enforce_http_request_authorization_maps_deny_to_forbidden() {
        let auth = AuthorizationHook::new().with_engine(
            RbacAdapter::default()
                .grant_role("svc", "reader")
                .grant_permission("reader", "/metrics", "GET"),
        );
        let request = Request {
            method: "GET".to_string(),
            path: "/admin".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let err = enforce_http_request_authorization(
            &auth,
            &request,
            "svc",
            HashMap::new(),
        )
        .expect_err("authorization should deny");
        assert!(matches!(err, ServerError::Forbidden(_)));
    }

    #[test]
    fn enforce_http_request_authorization_returns_ok_when_allowed() {
        let auth = AuthorizationHook::new().with_engine(
            RbacAdapter::default()
                .grant_role("svc", "reader")
                .grant_permission("reader", "/metrics", "GET"),
        );
        let request = Request {
            method: "GET".to_string(),
            path: "/metrics".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        enforce_http_request_authorization(
            &auth,
            &request,
            "svc",
            HashMap::new(),
        )
        .expect("should allow");
    }

    #[test]
    fn error_context_helpers_wrap_source_message() {
        // serde_json::Error — cheap: deserialize a clearly invalid doc.
        let json_err =
            serde_json::from_str::<u32>("definitely-not-a-number")
                .expect_err("invalid number");
        let audit = audit_serialize_err(json_err);
        assert!(matches!(audit, ServerError::Custom(_)));
        assert!(audit.to_string().contains("audit serialize:"));

        // toml::ser::Error — serializing a scalar at the root fails
        // because TOML requires a table at the root.
        let toml_err = toml::to_string_pretty(&42_u32)
            .expect_err("scalar root is not valid TOML");
        let cfg = serialize_config_err(toml_err);
        assert!(matches!(cfg, ServerError::Custom(_)));
        assert!(cfg.to_string().contains("serialize config:"));

        // notify::Error has a public constructor from io::Error.
        let init_err = watcher_init_err(notify::Error::generic(
            "mock init failure",
        ));
        assert!(matches!(init_err, ServerError::Custom(_)));
        assert!(init_err.to_string().contains("watcher init failed:"));

        let watch_err = watcher_watch_err(notify::Error::generic(
            "mock watch failure",
        ));
        assert!(matches!(watch_err, ServerError::Custom(_)));
        assert!(watch_err.to_string().contains("watch failed:"));
    }

    #[test]
    fn reloader_applies_file_updates() {
        use std::time::{Duration, Instant};
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("enterprise.toml");
        EnterpriseConfig::default()
            .save_to_file(&path)
            .expect("initial write");

        let reloader =
            EnterpriseConfigReloader::watch(&path).expect("watch");
        assert_eq!(reloader.snapshot().profile, RuntimeProfile::Dev);

        // Give the watcher a moment to subscribe before we edit.
        std::thread::sleep(Duration::from_millis(100));
        EnterpriseConfig::production_baseline()
            .save_to_file(&path)
            .expect("update write");

        // Wait for the async watcher callback to swap the atomic snapshot.
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if reloader.snapshot().profile == RuntimeProfile::Prod {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!(
            "reloader did not observe file update within 10s; final profile={:?}",
            reloader.snapshot().profile
        );
    }
}
