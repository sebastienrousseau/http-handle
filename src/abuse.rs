// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Abuse protection beyond the per-minute rate limit: per-client
//! bursts, per-route quotas, and global load shedding that spares
//! trusted clients.
//!
//! An [`AbusePolicy`], set with
//! [`ServerBuilder::abuse_policy`](crate::ServerBuilder::abuse_policy),
//! is checked for every request whose client address is known, before
//! the WAF and routing. The checks run in this order and the first
//! limit exceeded answers:
//!
//! 1. **Burst**: at most [`Limit::max_requests`] per client in
//!    [`Limit::window_ms`]. `429 Too Many Requests`.
//! 2. **Route quota**: per client, for requests that match the named
//!    [`Route`](crate::routing::Route). `429 Too Many Requests`.
//! 3. **Global**: all clients together. Untrusted clients get
//!    `503 Service Unavailable` once it is exceeded; clients in
//!    [`AbusePolicy::trusted`] are still served.
//!
//! Routes listed in [`AbusePolicy::exempt_routes`] skip every check.
//! Counting uses fixed windows. Each refusal logs a `warn` line naming
//! the reason, the scope and the client, carries a `Retry-After`
//! header, and is counted in [`stats`] and on `/metrics`.
//!
//! # Examples
//!
//! ```rust
//! use http_handle::abuse::{AbusePolicy, Limit};
//! use http_handle::Server;
//!
//! let policy = AbusePolicy {
//!     burst: Some(Limit { max_requests: 20, window_ms: 1_000 }),
//!     trusted: vec!["10.0.0.0/8".into()],
//!     ..AbusePolicy::default()
//! };
//! let server = Server::builder()
//!     .address("127.0.0.1:0")
//!     .document_root(".")
//!     .abuse_policy(policy)
//!     .expect("valid policy")
//!     .build()
//!     .expect("server");
//! assert!(server.abuse_policy().is_some());
//! ```

use crate::response::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

/// A request budget over a fixed window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limit {
    /// Requests allowed per window.
    pub max_requests: u32,
    /// Window length in milliseconds.
    pub window_ms: u64,
}

/// A per-client quota for one route.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteQuota {
    /// Name of the route, as given to [`Route::new`](crate::routing::Route::new).
    pub route: String,
    /// The budget.
    pub limit: Limit,
}

/// Abuse controls. Every part is optional.
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(default)]
pub struct AbusePolicy {
    /// Per-client burst limit.
    pub burst: Option<Limit>,
    /// Per-client quotas for named routes.
    pub route_quotas: Vec<RouteQuota>,
    /// Limit across all clients; trusted clients are exempt.
    pub global: Option<Limit>,
    /// Trusted clients: IP addresses or CIDR blocks such as
    /// `10.0.0.0/8` or `fd00::/8`.
    pub trusted: Vec<String>,
    /// Routes the policy does not apply to.
    pub exempt_routes: Vec<String>,
}

/// Why an abuse policy was rejected.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AbuseError {
    /// A limit allows no requests or has an empty window.
    #[error(
        "{0}: max_requests and window_ms must be greater than zero"
    )]
    InvalidLimit(String),
    /// A trusted entry is not an IP address or CIDR block.
    #[error("invalid trusted address `{0}`")]
    InvalidTrusted(String),
    /// Two quotas name the same route.
    #[error("duplicate quota for route `{0}`")]
    DuplicateQuota(String),
}

impl AbusePolicy {
    /// Checks limits, trusted entries and quotas.
    ///
    /// # Errors
    ///
    /// Returns an [`AbuseError`] for a zero limit, an unparsable
    /// trusted entry, or two quotas for one route.
    pub fn validate(&self) -> Result<(), AbuseError> {
        self.validate_limits()?;
        if let Some(bad) =
            self.trusted.iter().find(|t| parse_cidr(t).is_none())
        {
            return Err(AbuseError::InvalidTrusted(bad.clone()));
        }
        for (i, quota) in self.route_quotas.iter().enumerate() {
            if self.route_quotas[..i]
                .iter()
                .any(|q| q.route == quota.route)
            {
                return Err(AbuseError::DuplicateQuota(
                    quota.route.clone(),
                ));
            }
        }
        Ok(())
    }

    fn validate_limits(&self) -> Result<(), AbuseError> {
        let named =
            |name: &str, limit: &Limit| (name.to_string(), *limit);
        let limits = self
            .burst
            .iter()
            .map(|l| named("burst", l))
            .chain(self.global.iter().map(|l| named("global", l)))
            .chain(self.route_quotas.iter().map(|q| {
                named(&format!("route `{}`", q.route), &q.limit)
            }));
        for (name, limit) in limits {
            if limit.max_requests == 0 || limit.window_ms == 0 {
                return Err(AbuseError::InvalidLimit(name));
            }
        }
        Ok(())
    }

    fn is_trusted(&self, ip: IpAddr) -> bool {
        self.trusted
            .iter()
            .filter_map(|t| parse_cidr(t))
            .any(|(net, bits)| in_network(ip, net, bits))
    }
}

/// Which limit refused a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbuseReason {
    /// Per-client burst limit.
    Burst,
    /// Per-client route quota.
    RouteQuota,
    /// Global limit.
    Global,
}

/// A refused request, as logged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbuseEvent {
    /// The limit that refused it.
    pub reason: AbuseReason,
    /// What was counted: `client`, `route:<name>` or `global`.
    pub scope: String,
    /// The client address.
    pub client: IpAddr,
    /// Seconds until the window resets, at least 1.
    pub retry_after_secs: u64,
}

impl AbuseEvent {
    /// One `key=value` line, the form written to the log.
    pub fn to_log_line(&self) -> String {
        let (reason, action) = match self.reason {
            AbuseReason::Burst => ("burst", "throttled"),
            AbuseReason::RouteQuota => ("route_quota", "throttled"),
            AbuseReason::Global => ("global", "shed"),
        };
        format!(
            "abuse reason={reason} scope={} action={action} client={}",
            self.scope, self.client
        )
    }

    /// The response sent instead of the resource.
    pub(crate) fn response(&self) -> Response {
        let mut response = match self.reason {
            AbuseReason::Global => Response::new(
                503,
                "SERVICE UNAVAILABLE",
                b"Server busy".to_vec(),
            ),
            _ => Response::new(
                429,
                "TOO MANY REQUESTS",
                b"Rate limit exceeded".to_vec(),
            ),
        };
        response.add_header("Content-Type", "text/plain");
        response.add_header(
            "Retry-After",
            &self.retry_after_secs.to_string(),
        );
        response
    }
}

/// One limit to check: reason, scope, counter key, budget.
type Check = (AbuseReason, String, Option<IpAddr>, Limit);

/// Fixed-window counters keyed by scope and client.
type Windows = HashMap<(String, Option<IpAddr>), (Instant, u32)>;

/// Entries kept before expired windows are pruned.
const MAX_TRACKED: usize = 65_536;

/// An [`AbusePolicy`] with its counters.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "AbusePolicy", into = "AbusePolicy")]
pub(crate) struct AbuseGuard {
    policy: AbusePolicy,
    windows: Arc<Mutex<Windows>>,
}

impl PartialEq for AbuseGuard {
    fn eq(&self, other: &Self) -> bool {
        self.policy == other.policy
    }
}

impl Eq for AbuseGuard {}

impl TryFrom<AbusePolicy> for AbuseGuard {
    type Error = AbuseError;

    fn try_from(policy: AbusePolicy) -> Result<Self, Self::Error> {
        policy.validate()?;
        Ok(Self {
            policy,
            windows: Arc::default(),
        })
    }
}

impl From<AbuseGuard> for AbusePolicy {
    fn from(guard: AbuseGuard) -> Self {
        guard.policy
    }
}

impl AbuseGuard {
    pub(crate) fn policy(&self) -> &AbusePolicy {
        &self.policy
    }

    /// Counts the request and returns the event when a limit refuses
    /// it. `route` is the name of the matched route, if any.
    pub(crate) fn check(
        &self,
        client: IpAddr,
        route: Option<&str>,
    ) -> Option<AbuseEvent> {
        let policy = &self.policy;
        if route.is_some_and(|r| {
            policy.exempt_routes.iter().any(|e| e == r)
        }) {
            return None;
        }
        let now = Instant::now();
        let mut windows = match self.windows.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        prune(&mut windows, now, self.longest_window());
        let checks = self.checks(client, route);
        checks.into_iter().flatten().find_map(
            |(reason, scope, key, limit)| {
                let retry = hit(
                    &mut windows,
                    (scope.clone(), key),
                    limit,
                    now,
                )?;
                Some(AbuseEvent {
                    reason,
                    scope,
                    client,
                    retry_after_secs: retry,
                })
            },
        )
    }

    /// The longest configured window.
    fn longest_window(&self) -> Duration {
        let policy = &self.policy;
        let longest = policy
            .burst
            .iter()
            .chain(policy.global.iter())
            .chain(policy.route_quotas.iter().map(|q| &q.limit))
            .map(|l| l.window_ms)
            .max()
            .unwrap_or(0);
        Duration::from_millis(longest)
    }

    /// Number of live counter windows.
    #[cfg(test)]
    pub(crate) fn tracked(&self) -> usize {
        self.windows.lock().map_or(0, |w| w.len())
    }

    /// The limits that apply to `client` on `route`, in check order,
    /// each with its reason, scope and counter key.
    fn checks(
        &self,
        client: IpAddr,
        route: Option<&str>,
    ) -> [Option<Check>; 3] {
        let policy = &self.policy;
        let quota = route.and_then(|r| {
            policy.route_quotas.iter().find(|q| q.route == r)
        });
        [
            policy.burst.map(|l| {
                (
                    AbuseReason::Burst,
                    "client".to_string(),
                    Some(client),
                    l,
                )
            }),
            quota.map(|q| {
                let scope = format!("route:{}", q.route);
                (AbuseReason::RouteQuota, scope, Some(client), q.limit)
            }),
            policy.global.filter(|_| !policy.is_trusted(client)).map(
                |l| {
                    (AbuseReason::Global, "global".to_string(), None, l)
                },
            ),
        ]
    }
}

/// Counts one request in the window for `key`. Returns the seconds to
/// wait when the window is already full.
fn hit(
    windows: &mut Windows,
    key: (String, Option<IpAddr>),
    limit: Limit,
    now: Instant,
) -> Option<u64> {
    let length = Duration::from_millis(limit.window_ms);
    let entry = windows.entry(key).or_insert((now, 0));
    if now.duration_since(entry.0) >= length {
        *entry = (now, 0);
    }
    if entry.1 >= limit.max_requests {
        let left = length.saturating_sub(now.duration_since(entry.0));
        return Some(left.as_secs_f64().ceil().max(1.0) as u64);
    }
    entry.1 += 1;
    None
}

/// Bounds the table once it reaches [`MAX_TRACKED`]: drops windows
/// older than `longest`, and if every window is still live (a flood
/// of distinct clients) starts again from an empty table, so memory
/// and per-request work stay bounded.
fn prune(windows: &mut Windows, now: Instant, longest: Duration) {
    if windows.len() < MAX_TRACKED {
        return;
    }
    windows
        .retain(|_, (start, _)| now.duration_since(*start) < longest);
    if windows.len() >= MAX_TRACKED / 2 {
        log::warn!(
            "abuse counters reset: {} live windows exceed the cap",
            windows.len()
        );
        windows.clear();
    }
}

/// `10.0.0.0/8` to (address, 8); a bare address gets a full mask.
fn parse_cidr(value: &str) -> Option<(IpAddr, u8)> {
    let (address, bits) = match value.split_once('/') {
        Some((address, bits)) => {
            (address, Some(bits.parse::<u8>().ok()?))
        }
        None => (value, None),
    };
    let address: IpAddr = address.trim().parse().ok()?;
    let max = if address.is_ipv4() { 32 } else { 128 };
    let bits = bits.unwrap_or(max);
    (bits <= max).then_some((address, bits))
}

fn in_network(ip: IpAddr, net: IpAddr, bits: u8) -> bool {
    let (ip, net, width) = match (ip, net) {
        (IpAddr::V4(a), IpAddr::V4(b)) => {
            (u128::from(a.to_bits()), u128::from(b.to_bits()), 32)
        }
        (IpAddr::V6(a), IpAddr::V6(b)) => {
            (a.to_bits(), b.to_bits(), 128)
        }
        _ => return false,
    };
    let shift = width - u32::from(bits);
    shift >= width || (ip >> shift) == (net >> shift)
}

static BURST: AtomicUsize = AtomicUsize::new(0);
static ROUTE_QUOTA: AtomicUsize = AtomicUsize::new(0);
static GLOBAL: AtomicUsize = AtomicUsize::new(0);

/// Counts of refused requests since the process started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AbuseStats {
    /// Refused by the burst limit.
    pub burst: usize,
    /// Refused by a route quota.
    pub route_quota: usize,
    /// Shed by the global limit.
    pub global: usize,
}

/// Returns the abuse counters.
pub fn stats() -> AbuseStats {
    AbuseStats {
        burst: BURST.load(Ordering::Relaxed),
        route_quota: ROUTE_QUOTA.load(Ordering::Relaxed),
        global: GLOBAL.load(Ordering::Relaxed),
    }
}

/// Prometheus text lines for the `/metrics` endpoint.
pub(crate) fn metrics_lines() -> String {
    let s = stats();
    format!(
        "http_handle_abuse_total{{reason=\"burst\"}} {}\nhttp_handle_abuse_total{{reason=\"route_quota\"}} {}\nhttp_handle_abuse_total{{reason=\"global\"}} {}\n",
        s.burst, s.route_quota, s.global
    )
}

/// Logs and counts `event`.
pub(crate) fn record(event: &AbuseEvent) {
    log::warn!("{}", event.to_log_line());
    let counter = match event.reason {
        AbuseReason::Burst => &BURST,
        AbuseReason::RouteQuota => &ROUTE_QUOTA,
        AbuseReason::Global => &GLOBAL,
    };
    let _ = counter.fetch_add(1, Ordering::Relaxed);
}
