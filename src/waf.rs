// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Baseline web application firewall: request rules matched before a
//! request is routed or served.
//!
//! A [`WafPolicy`] holds [`WafRule`]s, each a case-insensitive regular
//! expression over one part of the request ([`WafTarget`]). The request
//! target is checked both as received and percent-decoded, so
//! `%2e%2e%2f` matches a rule written for `../`. The first rule that
//! matches decides:
//!
//! - in [`WafMode::Block`] the request gets `403 Forbidden`;
//! - in [`WafMode::ReportOnly`] it is served as usual.
//!
//! Either way a [`WafEvent`] is logged at `warn` level through the
//! `log` crate with the rule id, the target, the action and the
//! request line, and counted in [`stats`] and on `/metrics`.
//!
//! [`WafPolicy::baseline`] gives a starting rule set for path
//! traversal, null bytes, VCS and environment files, script injection,
//! SQL injection and shell command injection. It is a coarse filter,
//! not a substitute for validating input where it is used.
//!
//! # Examples
//!
//! ```rust
//! use http_handle::waf::{WafMode, WafPolicy, WafRule, WafTarget};
//! use http_handle::Server;
//!
//! let mut rules = WafPolicy::baseline_rules();
//! rules.push(WafRule::new("no-admin", WafTarget::Uri, r"^/admin(/|$)"));
//! let waf = WafPolicy::new(WafMode::Block, rules).expect("valid rules");
//!
//! let server = Server::builder()
//!     .address("127.0.0.1:0")
//!     .document_root(".")
//!     .waf(waf)
//!     .build()
//!     .expect("server");
//! assert!(server.waf().is_some());
//! ```

use crate::request::Request;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

/// What happens when a rule matches.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WafMode {
    /// Deny the request with `403 Forbidden`.
    #[default]
    Block,
    /// Serve the request and record the match.
    ReportOnly,
}

/// The part of the request a rule inspects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WafTarget {
    /// The request target (path and query), raw and percent-decoded.
    Uri,
    /// The request method.
    Method,
    /// One header, by name (case-insensitive).
    Header(String),
    /// Every header value.
    AnyHeader,
}

/// One rule definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafRule {
    /// Unique id, reported in events.
    pub id: String,
    /// What the rule inspects.
    pub target: WafTarget,
    /// Regular expression, matched case-insensitively.
    pub pattern: String,
}

impl WafRule {
    /// Creates a rule.
    pub fn new(id: &str, target: WafTarget, pattern: &str) -> Self {
        Self {
            id: id.to_string(),
            target,
            pattern: pattern.to_string(),
        }
    }
}

/// Why a rule set was rejected.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WafError {
    /// A rule has an empty id.
    #[error("WAF rule id must not be empty")]
    EmptyId,
    /// Two rules share an id.
    #[error("duplicate WAF rule id `{0}`")]
    DuplicateId(String),
    /// A header target names no header.
    #[error("WAF rule `{0}`: header target needs a header name")]
    EmptyHeaderName(String),
    /// A pattern is not a valid regular expression.
    #[error("WAF rule `{id}`: invalid pattern: {reason}")]
    InvalidPattern {
        /// Rule id.
        id: String,
        /// The regex compiler's message.
        reason: String,
    },
}

/// The action taken for a matched rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WafAction {
    /// The request was denied.
    Blocked,
    /// The request was served and the match recorded.
    Reported,
}

/// A rule match, as logged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WafEvent {
    /// Id of the rule that matched.
    pub rule_id: String,
    /// The inspected part, for example `uri` or `header:user-agent`.
    pub target: String,
    /// What was done.
    pub action: WafAction,
    /// Request method.
    pub method: String,
    /// Request target as received.
    pub path: String,
}

impl WafEvent {
    /// One `key=value` line, the form written to the log.
    pub fn to_log_line(&self) -> String {
        let action = match self.action {
            WafAction::Blocked => "blocked",
            WafAction::Reported => "reported",
        };
        format!(
            "waf rule={} target={} action={action} method={} path={:?}",
            self.rule_id, self.target, self.method, self.path
        )
    }
}

#[derive(Clone, Debug)]
struct CompiledRule {
    rule: WafRule,
    regex: Regex,
}

/// Configuration form of a [`WafPolicy`], used for serde.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WafConfig {
    /// Block or report.
    #[serde(default)]
    pub mode: WafMode,
    /// Rules, tried in order.
    pub rules: Vec<WafRule>,
}

/// A validated rule set and its mode.
///
/// Serialises as a [`WafConfig`]; deserialising compiles and validates
/// the rules again.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "WafConfig", into = "WafConfig")]
pub struct WafPolicy {
    mode: WafMode,
    rules: Vec<CompiledRule>,
}

impl PartialEq for WafPolicy {
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode && self.rules().eq(other.rules())
    }
}

impl Eq for WafPolicy {}

impl WafPolicy {
    /// Compiles and validates `rules`.
    ///
    /// # Errors
    ///
    /// Returns a [`WafError`] for an empty or duplicate id, a header
    /// target without a name, or a pattern that does not compile.
    pub fn new(
        mode: WafMode,
        rules: Vec<WafRule>,
    ) -> Result<Self, WafError> {
        let mut compiled: Vec<CompiledRule> =
            Vec::with_capacity(rules.len());
        for rule in rules {
            validate(&rule, &compiled)?;
            let regex = RegexBuilder::new(&rule.pattern)
                .case_insensitive(true)
                .build()
                .map_err(|e| WafError::InvalidPattern {
                    id: rule.id.clone(),
                    reason: e.to_string(),
                })?;
            compiled.push(CompiledRule { rule, regex });
        }
        Ok(Self {
            mode,
            rules: compiled,
        })
    }

    /// The baseline rule set in the given mode.
    pub fn baseline(mode: WafMode) -> Self {
        Self::new(mode, Self::baseline_rules())
            .expect("baseline rules are valid")
    }

    /// The baseline rules, to extend before calling [`WafPolicy::new`].
    pub fn baseline_rules() -> Vec<WafRule> {
        use WafTarget::{AnyHeader, Uri};
        [
            ("path-traversal", Uri, r"(^|[/\\])\.\.([/\\]|$)"),
            ("null-byte", Uri, r"\x00"),
            ("sensitive-file", Uri, r"/\.(git|svn|hg|env|htaccess|htpasswd)(/|$)"),
            ("script-injection", Uri, r"<\s*script|javascript:|on(error|load)\s*="),
            ("sql-injection", Uri, r"\bunion\b.+\bselect\b|'\s*or\s+'?\d+'?\s*=\s*'?\d+|;\s*drop\s+table\b"),
            ("command-injection", Uri, r"[;|`]\s*(cat|ls|id|wget|curl|sh|bash)\b|\$\("),
            ("header-injection", AnyHeader, r"[\r\n]|\$\{jndi:"),
        ]
        .into_iter()
        .map(|(id, target, pattern)| WafRule::new(id, target, pattern))
        .collect()
    }

    /// The mode.
    pub fn mode(&self) -> WafMode {
        self.mode
    }

    /// The rules, in matching order.
    pub fn rules(&self) -> impl Iterator<Item = &WafRule> {
        self.rules.iter().map(|c| &c.rule)
    }

    /// The first rule `request` matches, as an event with the action
    /// this policy's mode takes. Does not log or count.
    pub fn inspect(&self, request: &Request) -> Option<WafEvent> {
        let decoded = percent_decode(request.path());
        let compiled = self
            .rules
            .iter()
            .find(|c| matches(c, request, &decoded))?;
        Some(WafEvent {
            rule_id: compiled.rule.id.clone(),
            target: target_label(&compiled.rule.target),
            action: match self.mode {
                WafMode::Block => WafAction::Blocked,
                WafMode::ReportOnly => WafAction::Reported,
            },
            method: request.method().to_string(),
            path: request.path().to_string(),
        })
    }
}

impl TryFrom<WafConfig> for WafPolicy {
    type Error = WafError;

    fn try_from(config: WafConfig) -> Result<Self, Self::Error> {
        Self::new(config.mode, config.rules)
    }
}

impl From<WafPolicy> for WafConfig {
    fn from(policy: WafPolicy) -> Self {
        Self {
            mode: policy.mode,
            rules: policy.rules.into_iter().map(|c| c.rule).collect(),
        }
    }
}

static BLOCKED: AtomicUsize = AtomicUsize::new(0);
static REPORTED: AtomicUsize = AtomicUsize::new(0);

/// Counts of WAF matches since the process started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WafStats {
    /// Requests denied.
    pub blocked: usize,
    /// Matches recorded in report-only mode.
    pub reported: usize,
}

/// Returns the WAF counters.
pub fn stats() -> WafStats {
    WafStats {
        blocked: BLOCKED.load(Ordering::Relaxed),
        reported: REPORTED.load(Ordering::Relaxed),
    }
}

/// Prometheus text lines for the `/metrics` endpoint.
pub(crate) fn metrics_lines() -> String {
    let s = stats();
    format!(
        "http_handle_waf_total{{action=\"blocked\"}} {}\nhttp_handle_waf_total{{action=\"reported\"}} {}\n",
        s.blocked, s.reported
    )
}

/// Logs and counts `event`.
pub(crate) fn record(event: &WafEvent) {
    log::warn!("{}", event.to_log_line());
    let counter = match event.action {
        WafAction::Blocked => &BLOCKED,
        WafAction::Reported => &REPORTED,
    };
    let _ = counter.fetch_add(1, Ordering::Relaxed);
}

fn validate(
    rule: &WafRule,
    earlier: &[CompiledRule],
) -> Result<(), WafError> {
    if rule.id.is_empty() {
        return Err(WafError::EmptyId);
    }
    if earlier.iter().any(|c| c.rule.id == rule.id) {
        return Err(WafError::DuplicateId(rule.id.clone()));
    }
    match &rule.target {
        WafTarget::Header(name) if name.is_empty() => {
            Err(WafError::EmptyHeaderName(rule.id.clone()))
        }
        _ => Ok(()),
    }
}

fn matches(
    compiled: &CompiledRule,
    request: &Request,
    decoded: &str,
) -> bool {
    let regex = &compiled.regex;
    match &compiled.rule.target {
        WafTarget::Uri => {
            regex.is_match(request.path()) || regex.is_match(decoded)
        }
        WafTarget::Method => regex.is_match(request.method()),
        WafTarget::Header(name) => {
            request.header(name).is_some_and(|v| regex.is_match(v))
        }
        WafTarget::AnyHeader => {
            request.headers().iter().any(|(_, v)| regex.is_match(v))
        }
    }
}

fn target_label(target: &WafTarget) -> String {
    match target {
        WafTarget::Uri => "uri".into(),
        WafTarget::Method => "method".into(),
        WafTarget::Header(name) => {
            format!("header:{}", name.to_ascii_lowercase())
        }
        WafTarget::AnyHeader => "any_header".into(),
    }
}

/// Decodes `%XX` escapes; invalid escapes are kept as written.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| {
            u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()
        });
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
