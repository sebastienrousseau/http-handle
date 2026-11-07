// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

use super::{header, path_str, request, site};
use crate::abuse::{
    AbuseError, AbuseEvent, AbusePolicy, AbuseReason, Limit,
    RouteQuota, stats,
};
use crate::routing::{Route, Router};
use crate::server::{Server, build_response_for_request_with_metrics};
use std::net::IpAddr;
use tempfile::TempDir;

const CLIENT: &str = "192.0.2.10";
const OTHER: &str = "192.0.2.20";
const TRUSTED: &str = "10.1.2.3";

fn ip(value: &str) -> IpAddr {
    value.parse().expect("ip")
}

fn limit(max_requests: u32) -> Limit {
    Limit {
        max_requests,
        window_ms: 60_000,
    }
}

/// A server with an `api` route, a `health` route, and `policy`.
fn server(policy: AbusePolicy) -> (TempDir, Server) {
    let dir = site(&[
        ("api/x.txt", "api"),
        ("health/x.txt", "ok"),
        ("x.txt", "site"),
    ]);
    let router = Router::new(vec![
        Route::new("api", "/api", dir.path().join("api")),
        Route::new("health", "/health", dir.path().join("health")),
        Route::new("site", "/", dir.path()),
    ])
    .expect("router");
    let server = Server::builder()
        .address("127.0.0.1:0")
        .document_root(path_str(dir.path()))
        .router(router)
        .abuse_policy(policy)
        .expect("valid policy")
        .build()
        .expect("server");
    (dir, server)
}

fn status(server: &Server, path: &str, client: &str) -> u16 {
    build_response_for_request_with_metrics(
        server,
        &request("GET", path, &[]),
        Some(ip(client)),
    )
    .status_code
}

#[test]
fn burst_from_one_client_is_throttled_others_are_not() {
    let (_dir, server) = server(AbusePolicy {
        burst: Some(limit(3)),
        ..AbusePolicy::default()
    });
    let before = stats().burst;
    let codes: Vec<u16> =
        (0..4).map(|_| status(&server, "/x.txt", CLIENT)).collect();
    assert_eq!(codes, [200, 200, 200, 429]);
    assert_eq!(status(&server, "/x.txt", OTHER), 200);
    let refused = build_response_for_request_with_metrics(
        &server,
        &request("GET", "/x.txt", &[]),
        Some(ip(CLIENT)),
    );
    assert!(
        header(&refused, "retry-after")
            .is_some_and(|s| s.parse::<u64>().is_ok_and(|n| n >= 1))
    );
    assert!(stats().burst > before);
}

#[test]
fn route_quota_limits_only_that_route() {
    let policy = AbusePolicy {
        route_quotas: vec![RouteQuota {
            route: "api".into(),
            limit: limit(2),
        }],
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy);
    let before = stats().route_quota;
    assert_eq!(status(&server, "/api/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/api/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/api/x.txt", CLIENT), 429);
    assert_eq!(status(&server, "/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/api/x.txt", OTHER), 200);
    assert!(stats().route_quota > before);
}

#[test]
fn global_pressure_sheds_untrusted_clients_first() {
    let policy = AbusePolicy {
        global: Some(limit(2)),
        trusted: vec!["10.0.0.0/8".into()],
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy);
    let before = stats().global;
    assert_eq!(status(&server, "/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/x.txt", OTHER), 200);
    assert_eq!(status(&server, "/x.txt", CLIENT), 503);
    assert_eq!(status(&server, "/x.txt", TRUSTED), 200);
    assert_eq!(status(&server, "/x.txt", TRUSTED), 200);
    assert!(stats().global > before);
}

#[test]
fn exempt_route_skips_every_check() {
    let policy = AbusePolicy {
        burst: Some(limit(1)),
        exempt_routes: vec!["health".into()],
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy);
    for _ in 0..5 {
        assert_eq!(status(&server, "/health/x.txt", CLIENT), 200);
    }
    assert_eq!(status(&server, "/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/x.txt", CLIENT), 429);
}

#[test]
fn unknown_client_address_is_not_limited() {
    let (_dir, server) = server(AbusePolicy {
        burst: Some(limit(1)),
        ..AbusePolicy::default()
    });
    for _ in 0..3 {
        let response = build_response_for_request_with_metrics(
            &server,
            &request("GET", "/x.txt", &[]),
            None,
        );
        assert_eq!(response.status_code, 200);
    }
}

#[test]
fn events_name_reason_scope_and_client() {
    let event = AbuseEvent {
        reason: AbuseReason::RouteQuota,
        scope: "route:api".into(),
        client: ip(CLIENT),
        retry_after_secs: 3,
    };
    assert_eq!(
        event.to_log_line(),
        "abuse reason=route_quota scope=route:api action=throttled client=192.0.2.10"
    );
}

#[test]
fn invalid_policies_are_rejected() {
    let zero = AbusePolicy {
        burst: Some(Limit {
            max_requests: 0,
            window_ms: 1,
        }),
        ..AbusePolicy::default()
    };
    assert_eq!(
        zero.validate(),
        Err(AbuseError::InvalidLimit("burst".into()))
    );
    for bad in ["10.0.0.0/33", "not-an-ip", "::1/129"] {
        let policy = AbusePolicy {
            trusted: vec![bad.into()],
            ..AbusePolicy::default()
        };
        assert_eq!(
            policy.validate(),
            Err(AbuseError::InvalidTrusted(bad.into()))
        );
    }
    let quota = RouteQuota {
        route: "api".into(),
        limit: limit(1),
    };
    let twice = AbusePolicy {
        route_quotas: vec![quota.clone(), quota],
        ..AbusePolicy::default()
    };
    assert_eq!(
        twice.validate(),
        Err(AbuseError::DuplicateQuota("api".into()))
    );
    let ok = AbusePolicy {
        trusted: vec![
            "::1".into(),
            "0.0.0.0/0".into(),
            "fd00::/8".into(),
        ],
        ..AbusePolicy::default()
    };
    assert_eq!(ok.validate(), Ok(()));
    assert!(Server::builder().abuse_policy(zero).is_err());
}

#[test]
fn metrics_endpoint_reports_abuse_reasons() {
    let body = String::from_utf8(
        crate::server::generate_metrics_response().body,
    )
    .expect("utf-8");
    for reason in ["burst", "route_quota", "global"] {
        assert!(
            body.contains(&format!(
                "http_handle_abuse_total{{reason=\"{reason}\"}}"
            )),
            "{reason}"
        );
    }
}

#[test]
fn policy_survives_server_serde_round_trip() {
    let policy = AbusePolicy {
        burst: Some(limit(5)),
        trusted: vec!["fd00::/8".into()],
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy.clone());
    assert_eq!(server.abuse_policy(), Some(&policy));
    let json = serde_json::to_string(&server).expect("serialize");
    let back: Server =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.abuse_policy(), Some(&policy));
    assert_eq!(back.policies_for_test(), server.policies_for_test());
}

#[test]
fn ipv6_trust_does_not_cover_ipv4_clients() {
    let policy = AbusePolicy {
        global: Some(limit(1)),
        trusted: vec!["fd00::/8".into()],
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy);
    assert_eq!(status(&server, "/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/x.txt", CLIENT), 503);
    assert_eq!(status(&server, "/x.txt", "fd00::1"), 200);
    assert_eq!(status(&server, "/x.txt", "fe80::1"), 503);
}

#[test]
fn window_resets_after_it_expires() {
    let policy = AbusePolicy {
        burst: Some(Limit {
            max_requests: 1,
            window_ms: 50,
        }),
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy);
    assert_eq!(status(&server, "/x.txt", CLIENT), 200);
    assert_eq!(status(&server, "/x.txt", CLIENT), 429);
    std::thread::sleep(std::time::Duration::from_millis(80));
    assert_eq!(status(&server, "/x.txt", CLIENT), 200);
}

#[test]
fn counter_table_is_pruned_when_large() {
    let policy = AbusePolicy {
        burst: Some(limit(1)),
        ..AbusePolicy::default()
    };
    let (_dir, server) = server(policy);
    let guard =
        server.policies_for_test().abuse.as_ref().expect("guard");
    // A flood of distinct live clients must not grow the table past
    // its cap; limits keep working after the reset.
    for n in 0..70_000_u32 {
        let client =
            IpAddr::from(std::net::Ipv4Addr::from(0x0A00_0000 + n));
        assert!(guard.check(client, None).is_none());
        assert!(guard.tracked() <= 65_536);
    }
    assert!(guard.check(ip("10.1.17.111"), None).is_some());
}
