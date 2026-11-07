// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

use super::{header, path_str, request, respond, site};
use crate::conditional::{
    format_http_date, from_unix_secs, parse_http_date,
};
use crate::server::Server;
use std::fs::{File, FileTimes};
use tempfile::TempDir;

/// Mtime of the fixture file: 2024-02-29 12:34:56 UTC.
const MTIME: u64 = 1_709_210_096;
const MTIME_HTTP: &str = "Thu, 29 Feb 2024 12:34:56 GMT";

fn fixture(ttl: Option<u64>) -> (TempDir, Server) {
    let dir = site(&[("page.txt", "hello")]);
    let file = File::options()
        .write(true)
        .open(dir.path().join("page.txt"))
        .expect("open");
    file.set_times(
        FileTimes::new().set_modified(from_unix_secs(MTIME)),
    )
    .expect("set mtime");
    let mut builder = Server::builder()
        .address("127.0.0.1:0")
        .document_root(path_str(dir.path()));
    if let Some(ttl) = ttl {
        builder = builder.static_cache_ttl_secs(ttl);
    }
    (dir, builder.build().expect("server"))
}

fn etag(server: &Server) -> String {
    let response = respond(server, &request("GET", "/page.txt", &[]));
    header(&response, "etag").expect("etag").to_string()
}

#[test]
fn http_dates_round_trip() {
    assert_eq!(
        format_http_date(from_unix_secs(MTIME)).as_deref(),
        Some(MTIME_HTTP)
    );
    assert_eq!(parse_http_date(MTIME_HTTP), Some(MTIME));
    assert_eq!(
        parse_http_date("Thu, 01 Jan 1970 00:00:00 GMT"),
        Some(0)
    );
    assert_eq!(
        format_http_date(from_unix_secs(784_111_777)).as_deref(),
        Some("Sun, 06 Nov 1994 08:49:37 GMT")
    );
    for bad in [
        "",
        "yesterday",
        "Thu, 29 Feb 2024 12:34:56 UTC",
        "Thu, 32 Feb 2024 12:34:56 GMT",
        "Thu, 29 Foo 2024 12:34:56 GMT",
        "Thu, 29 Feb 2024 25:00:00 GMT",
    ] {
        assert_eq!(parse_http_date(bad), None, "{bad}");
    }
}

#[test]
fn full_response_carries_last_modified() {
    let (_dir, server) = fixture(None);
    let response = respond(&server, &request("GET", "/page.txt", &[]));
    assert_eq!(response.status_code, 200);
    assert_eq!(header(&response, "last-modified"), Some(MTIME_HTTP));
}

#[test]
fn if_modified_since_returns_304_when_not_newer() {
    let (_dir, server) = fixture(None);
    for since in [MTIME_HTTP, "Fri, 01 Mar 2024 00:00:00 GMT"] {
        let response = respond(
            &server,
            &request(
                "GET",
                "/page.txt",
                &[("If-Modified-Since", since)],
            ),
        );
        assert_eq!(response.status_code, 304, "{since}");
        assert!(response.body.is_empty());
        assert_eq!(
            header(&response, "last-modified"),
            Some(MTIME_HTTP)
        );
    }
    let older =
        [("If-Modified-Since", "Wed, 28 Feb 2024 00:00:00 GMT")];
    assert_eq!(
        respond(&server, &request("GET", "/page.txt", &older))
            .status_code,
        200
    );
    let garbage = [("If-Modified-Since", "not a date")];
    assert_eq!(
        respond(&server, &request("GET", "/page.txt", &garbage))
            .status_code,
        200
    );
}

#[test]
fn if_none_match_handles_lists_weak_tags_and_star() {
    let (_dir, server) = fixture(None);
    let tag = etag(&server);
    let strong = tag.trim_start_matches("W/").to_string();
    let list = format!("\"other\", {tag}");
    for candidate in [tag.as_str(), strong.as_str(), list.as_str(), "*"]
    {
        let response = respond(
            &server,
            &request(
                "GET",
                "/page.txt",
                &[("If-None-Match", candidate)],
            ),
        );
        assert_eq!(response.status_code, 304, "{candidate}");
        assert_eq!(header(&response, "etag"), Some(tag.as_str()));
    }
    let miss = [("If-None-Match", "\"other\"")];
    assert_eq!(
        respond(&server, &request("GET", "/page.txt", &miss))
            .status_code,
        200
    );
}

#[test]
fn if_none_match_takes_precedence_over_if_modified_since() {
    let (_dir, server) = fixture(None);
    let headers = [
        ("If-None-Match", "\"other\""),
        ("If-Modified-Since", MTIME_HTTP),
    ];
    assert_eq!(
        respond(&server, &request("GET", "/page.txt", &headers))
            .status_code,
        200
    );
}

#[test]
fn not_modified_keeps_cache_headers_and_applies_to_head() {
    let (_dir, server) = fixture(Some(60));
    let headers = [("If-Modified-Since", MTIME_HTTP)];
    let response =
        respond(&server, &request("HEAD", "/page.txt", &headers));
    assert_eq!(response.status_code, 304);
    assert_eq!(
        header(&response, "cache-control"),
        Some("public, max-age=60")
    );
}
