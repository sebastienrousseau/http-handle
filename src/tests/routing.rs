// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

use super::{header, path_str, request, respond, site};
use crate::routing::{Route, RouteError, Router};
use crate::server::Server;
use tempfile::TempDir;

struct Fixture {
    _default: TempDir,
    api: TempDir,
    docs: TempDir,
    server: Server,
}

/// Default root, an `api` root and a `docs` root, each with one file
/// that names it.
fn fixture(
    routes: impl FnOnce(&TempDir, &TempDir) -> Vec<Route>,
) -> Fixture {
    let default = site(&[("page.txt", "default")]);
    let api = site(&[("page.txt", "api"), ("v2/page.txt", "api v2")]);
    let docs =
        site(&[("page.txt", "docs"), ("index.html", "docs index")]);
    let router =
        Router::new(routes(&api, &docs)).expect("valid routes");
    let server = Server::builder()
        .address("127.0.0.1:0")
        .document_root(path_str(default.path()))
        .router(router)
        .build()
        .expect("server");
    Fixture {
        _default: default,
        api,
        docs,
        server,
    }
}

fn body(
    server: &Server,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> (u16, String) {
    let response = respond(server, &request(method, path, headers));
    (
        response.status_code,
        String::from_utf8_lossy(&response.body).into_owned(),
    )
}

#[test]
fn host_and_path_select_the_route_root_with_prefix_removed() {
    let f = fixture(|api, docs| {
        vec![
            Route::new("api", "/api", api.path())
                .host("api.example.com"),
            Route::new("docs", "/", docs.path()),
        ]
    });
    let host = [("Host", "API.example.com:8080")];
    assert_eq!(
        body(&f.server, "GET", "/api/page.txt", &host),
        (200, "api".into())
    );
    assert_eq!(
        body(&f.server, "GET", "/api/v2/page.txt", &host),
        (200, "api v2".into())
    );
    // Another host falls through to the catch-all route.
    let other = [("Host", "www.example.com")];
    assert_eq!(
        body(&f.server, "GET", "/page.txt", &other),
        (200, "docs".into())
    );
    // The prefix matches whole segments only.
    assert_eq!(body(&f.server, "GET", "/apipage.txt", &host).0, 404);
    // HEAD is routed the same way.
    assert_eq!(body(&f.server, "HEAD", "/api/page.txt", &host).0, 200);
    let _ = (&f.api, &f.docs);
}

#[test]
fn higher_priority_route_wins_over_more_specific_one() {
    let f = fixture(|api, docs| {
        vec![
            Route::new("api", "/api", api.path()),
            Route::new("docs", "/", docs.path()).priority(10),
        ]
    });
    assert_eq!(
        body(&f.server, "GET", "/api/page.txt", &[]),
        (404, "404 Not Found".into())
    );
    assert_eq!(
        body(&f.server, "GET", "/page.txt", &[]),
        (200, "docs".into())
    );
}

#[test]
fn longer_prefix_wins_at_equal_priority() {
    let f = fixture(|api, docs| {
        vec![
            Route::new("docs", "/", docs.path()),
            Route::new("api", "/api", api.path()),
        ]
    });
    assert_eq!(
        body(&f.server, "GET", "/api/page.txt", &[]),
        (200, "api".into())
    );
}

#[test]
fn unmatched_request_gets_plain_404_not_default_root() {
    let f =
        fixture(|api, _| vec![Route::new("api", "/api", api.path())]);
    let (status, text) = body(&f.server, "GET", "/page.txt", &[]);
    assert_eq!((status, text.as_str()), (404, "404 Not Found"));
    let response =
        respond(&f.server, &request("GET", "/page.txt", &[]));
    assert_eq!(header(&response, "content-type"), Some("text/plain"));
}

#[test]
fn method_and_header_constraints_must_hold() {
    let f = fixture(|api, _| {
        vec![
            Route::new("api", "/api", api.path())
                .method("get")
                .header("X-Tenant", "blue"),
        ]
    });
    let ok = [("X-Tenant", "blue")];
    assert_eq!(body(&f.server, "GET", "/api/page.txt", &ok).0, 200);
    assert_eq!(body(&f.server, "HEAD", "/api/page.txt", &ok).0, 404);
    assert_eq!(
        body(&f.server, "GET", "/api/page.txt", &[("X-Tenant", "red")])
            .0,
        404
    );
    assert_eq!(body(&f.server, "GET", "/api/page.txt", &[]).0, 404);
}

#[test]
fn query_string_does_not_break_prefix_match() {
    let f =
        fixture(|api, _| vec![Route::new("api", "/api", api.path())]);
    let route = f
        .server
        .router()
        .and_then(|r| r.select(&request("GET", "/api?x=1", &[])));
    assert_eq!(route.map(|r| r.name.as_str()), Some("api"));
}

#[test]
fn ambiguous_and_invalid_routes_are_rejected() {
    let root = site(&[]);
    let dir = root.path();
    let ambiguous = Router::new(vec![
        Route::new("a", "/x/", dir).method("GET"),
        Route::new("b", "/x", dir).method("get"),
    ]);
    assert_eq!(
        ambiguous,
        Err(RouteError::Ambiguous {
            first: "a".into(),
            second: "b".into()
        })
    );
    // Same constraints at different priorities are not ambiguous.
    assert!(
        Router::new(vec![
            Route::new("a", "/x", dir),
            Route::new("b", "/x", dir).priority(1),
        ])
        .is_ok()
    );
    assert_eq!(
        Router::new(vec![
            Route::new("a", "/", dir),
            Route::new("a", "/y", dir)
        ]),
        Err(RouteError::DuplicateName("a".into()))
    );
    assert!(matches!(
        Router::new(vec![Route::new("a", "x", dir)]),
        Err(RouteError::InvalidPathPrefix { .. })
    ));
    assert!(matches!(
        Router::new(vec![Route::new("a", "/", dir).method("GE T")]),
        Err(RouteError::InvalidConstraint {
            field: "method",
            ..
        })
    ));
    assert!(matches!(
        Router::new(vec![Route::new("a", "/", dir).host("")]),
        Err(RouteError::InvalidConstraint { field: "host", .. })
    ));
    assert!(matches!(
        Router::new(vec![Route::new("a", "/", dir.join("missing"))]),
        Err(RouteError::InvalidDocumentRoot { .. })
    ));
    assert_eq!(
        Router::new(vec![Route::new("", "/", dir)]),
        Err(RouteError::EmptyName)
    );
}

#[test]
fn router_round_trips_through_serde_and_revalidates() {
    let root = site(&[]);
    let router = Router::new(vec![Route::new("a", "/a", root.path())])
        .expect("router");
    let json = serde_json::to_string(&router).expect("serialize");
    let back: Router =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, router);
    let bad = json.replace("\"/a\"", "\"a\"");
    assert!(serde_json::from_str::<Router>(&bad).is_err());
}

#[test]
fn routes_cannot_escape_their_root() {
    let f =
        fixture(|api, _| vec![Route::new("api", "/api", api.path())]);
    assert_eq!(body(&f.server, "GET", "/api/../page.txt", &[]).0, 403);
}

#[test]
fn bare_prefix_and_ipv6_host_resolve() {
    let f = fixture(|api, _| {
        vec![Route::new("api", "/api", api.path()).host("[::1]")]
    });
    let names: Vec<&str> = f
        .server
        .router()
        .expect("router")
        .routes()
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(names, ["api"]);
    let host = [("Host", "[::1]:8080")];
    // `/api` itself maps to the route root, which has no index: 404 from that root.
    let route = f
        .server
        .router()
        .and_then(|r| r.select(&request("GET", "/api", &host)));
    assert_eq!(route.map(|r| r.name.as_str()), Some("api"));
    assert_eq!(body(&f.server, "GET", "/api", &host).0, 404);
    assert_eq!(
        body(&f.server, "GET", "/api/page.txt", &[("Host", "[::2]")]).0,
        404
    );
}
