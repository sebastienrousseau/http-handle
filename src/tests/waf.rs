// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

use super::{path_str, request, respond, site};
use crate::server::Server;
use crate::waf::{
    WafAction, WafError, WafMode, WafPolicy, WafRule, WafTarget, stats,
};
use tempfile::TempDir;

fn server(mode: WafMode) -> (TempDir, Server) {
    let dir = site(&[("page.txt", "ok")]);
    let server = Server::builder()
        .address("127.0.0.1:0")
        .document_root(path_str(dir.path()))
        .waf(WafPolicy::baseline(mode))
        .build()
        .expect("server");
    (dir, server)
}

#[test]
fn baseline_blocks_common_attacks() {
    let (_dir, server) = server(WafMode::Block);
    let attacks = [
        ("/%2e%2e/%2e%2e/etc/passwd", "path-traversal"),
        ("/page.txt%00.html", "null-byte"),
        ("/.git/config", "sensitive-file"),
        ("/.env", "sensitive-file"),
        ("/search?q=<script>alert(1)</script>", "script-injection"),
        ("/search?q=%3CScRiPt%3E", "script-injection"),
        (
            "/item?id=1%20UNION%20SELECT%20password%20FROM%20users",
            "sql-injection",
        ),
        ("/item?id=1'%20or%20'1'='1", "sql-injection"),
        ("/run?cmd=x;cat%20/etc/passwd", "command-injection"),
    ];
    let waf = server.waf().expect("waf");
    let before = stats().blocked;
    for (path, rule) in attacks {
        let req = request("GET", path, &[]);
        let event = waf.inspect(&req).expect(path);
        assert_eq!(
            (event.rule_id.as_str(), event.action),
            (rule, WafAction::Blocked),
            "{path}"
        );
        assert_eq!(respond(&server, &req).status_code, 403, "{path}");
    }
    let jndi = request(
        "GET",
        "/page.txt",
        &[("User-Agent", "${jndi:ldap://x}")],
    );
    assert_eq!(respond(&server, &jndi).status_code, 403);
    assert!(stats().blocked > before + attacks.len());
}

#[test]
fn clean_requests_pass() {
    let (_dir, server) = server(WafMode::Block);
    for path in [
        "/page.txt",
        "/page.txt?q=selection&order=desc",
        "/docs/union-station.html",
    ] {
        let req = request("GET", path, &[("User-Agent", "curl/8.0")]);
        assert_eq!(
            server.waf().expect("waf").inspect(&req),
            None,
            "{path}"
        );
    }
    assert_eq!(
        respond(&server, &request("GET", "/page.txt", &[])).status_code,
        200
    );
}

#[test]
fn report_only_serves_and_records() {
    let (_dir, server) = server(WafMode::ReportOnly);
    let req = request("GET", "/page.txt?x=<script>", &[]);
    let event =
        server.waf().expect("waf").inspect(&req).expect("match");
    assert_eq!(event.action, WafAction::Reported);
    let before = stats().reported;
    assert_ne!(respond(&server, &req).status_code, 403);
    assert!(stats().reported > before);
}

#[test]
fn events_name_rule_target_and_action() {
    let waf = WafPolicy::new(
        WafMode::Block,
        vec![WafRule::new(
            "bad-agent",
            WafTarget::Header("User-Agent".into()),
            "sqlmap",
        )],
    )
    .expect("valid");
    let event = waf
        .inspect(&request(
            "POST",
            "/x",
            &[("User-Agent", "SQLMap/1.7")],
        ))
        .expect("match");
    assert_eq!(
        event.to_log_line(),
        "waf rule=bad-agent target=header:user-agent action=blocked method=POST path=\"/x\""
    );
}

#[test]
fn invalid_rules_are_rejected() {
    let rule = |id: &str, target, pattern: &str| {
        WafRule::new(id, target, pattern)
    };
    assert!(matches!(
        WafPolicy::new(
            WafMode::Block,
            vec![rule("r", WafTarget::Uri, "(")]
        ),
        Err(WafError::InvalidPattern { .. })
    ));
    assert_eq!(
        WafPolicy::new(
            WafMode::Block,
            vec![rule("", WafTarget::Uri, "x")]
        ),
        Err(WafError::EmptyId)
    );
    assert_eq!(
        WafPolicy::new(
            WafMode::Block,
            vec![
                rule("r", WafTarget::Uri, "x"),
                rule("r", WafTarget::Method, "y")
            ]
        ),
        Err(WafError::DuplicateId("r".into()))
    );
    assert_eq!(
        WafPolicy::new(
            WafMode::Block,
            vec![rule("r", WafTarget::Header(String::new()), "x")]
        ),
        Err(WafError::EmptyHeaderName("r".into()))
    );
}

#[test]
fn policy_round_trips_through_serde_and_revalidates() {
    let waf = WafPolicy::baseline(WafMode::ReportOnly);
    let json = serde_json::to_string(&waf).expect("serialize");
    let back: WafPolicy =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, waf);
    let broken = r#"{"mode":"block","rules":[{"id":"r","target":"uri","pattern":"("}]}"#;
    assert!(serde_json::from_str::<WafPolicy>(broken).is_err());
}

#[test]
fn metrics_endpoint_reports_waf_actions() {
    let response = crate::server::generate_metrics_response();
    let body = String::from_utf8(response.body).expect("utf-8");
    assert!(body.contains("http_handle_waf_total{action=\"blocked\"}"));
    assert!(
        body.contains("http_handle_waf_total{action=\"reported\"}")
    );
}

#[test]
fn method_rules_match_the_method() {
    let waf = WafPolicy::new(
        WafMode::ReportOnly,
        vec![WafRule::new("no-trace", WafTarget::Method, "^TRACE$")],
    )
    .expect("valid");
    assert_eq!(waf.mode(), WafMode::ReportOnly);
    let event =
        waf.inspect(&request("TRACE", "/", &[])).expect("match");
    assert_eq!(
        (event.target.as_str(), event.action),
        ("method", WafAction::Reported)
    );
    assert_eq!(waf.inspect(&request("GET", "/", &[])), None);
}
