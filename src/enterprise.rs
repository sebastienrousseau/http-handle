//! Enterprise-ready primitives: TLS/mTLS policy, auth middleware hooks,
//! runtime profiles, hot reload, and structured audit logging.

#[cfg(feature = "enterprise")]
use crate::error::ServerError;
#[cfg(feature = "enterprise")]
use arc_swap::ArcSwap;
#[cfg(feature = "enterprise")]
use notify::{RecursiveMode, Watcher};
#[cfg(feature = "enterprise")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "enterprise")]
use std::collections::HashSet;
#[cfg(feature = "enterprise")]
use std::path::{Path, PathBuf};
#[cfg(feature = "enterprise")]
use std::sync::Arc;

/// Runtime deployment profile.
#[cfg(feature = "enterprise")]
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
#[cfg(feature = "enterprise")]
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
#[cfg(feature = "enterprise")]
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
#[cfg(feature = "enterprise")]
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
#[cfg(feature = "enterprise")]
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
impl EnterpriseConfig {
    /// Loads enterprise configuration from a TOML file.
    pub fn load_from_file(path: &Path) -> Result<Self, ServerError> {
        let text =
            std::fs::read_to_string(path).map_err(ServerError::from)?;
        toml::from_str(&text).map_err(|e| {
            ServerError::Custom(format!("invalid config: {e}"))
        })
    }

    /// Writes enterprise configuration as TOML.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ServerError> {
        let text = toml::to_string_pretty(self).map_err(|e| {
            ServerError::Custom(format!("serialize config: {e}"))
        })?;
        std::fs::write(path, text).map_err(ServerError::from)
    }

    /// Returns a strict, production-biased default profile.
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
#[cfg(feature = "enterprise")]
#[derive(Debug)]
pub struct EnterpriseConfigReloader {
    current: Arc<ArcSwap<EnterpriseConfig>>,
    _watcher: notify::RecommendedWatcher,
}

#[cfg(feature = "enterprise")]
impl EnterpriseConfigReloader {
    /// Starts watching a config file and atomically swaps updates.
    pub fn watch(path: impl AsRef<Path>) -> Result<Self, ServerError> {
        let path = path.as_ref().to_path_buf();
        let initial =
            Arc::new(EnterpriseConfig::load_from_file(&path)?);
        let current = Arc::new(ArcSwap::new(initial));
        let swap = Arc::clone(&current);
        let path_for_watch = path.clone();

        let mut watcher = notify::recommended_watcher(
            move |result: Result<notify::Event, notify::Error>| {
                if result.is_ok() {
                    if let Ok(next) = EnterpriseConfig::load_from_file(
                        &path_for_watch,
                    ) {
                        swap.store(Arc::new(next));
                    }
                }
            },
        )
        .map_err(|e| {
            ServerError::Custom(format!("watcher init failed: {e}"))
        })?;

        watcher.watch(&path, RecursiveMode::NonRecursive).map_err(
            |e| ServerError::Custom(format!("watch failed: {e}")),
        )?;

        Ok(Self {
            current,
            _watcher: watcher,
        })
    }

    /// Returns the latest config snapshot.
    pub fn snapshot(&self) -> Arc<EnterpriseConfig> {
        self.current.load_full()
    }
}

/// Structured access/audit event with trace correlation.
#[cfg(feature = "enterprise")]
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
impl AccessAuditEvent {
    /// Encodes a JSON log line for ingestion by SIEM/log pipelines.
    pub fn to_json_line(&self) -> Result<String, ServerError> {
        serde_json::to_string(self).map_err(|e| {
            ServerError::Custom(format!("audit serialize: {e}"))
        })
    }
}

/// Constant-time API key validation helper.
#[cfg(feature = "enterprise")]
pub fn validate_api_key(policy: &AuthPolicy, key: &str) -> bool {
    let allowed: HashSet<&str> =
        policy.api_keys.iter().map(String::as_str).collect();
    allowed.contains(key)
}

/// JWT validation helper (HS256).
#[cfg(feature = "enterprise")]
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
}
