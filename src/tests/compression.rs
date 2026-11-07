// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

use super::{header, path_str, request, respond, site};
use crate::compression::{CompressionPolicy, accepts, stats};
use crate::routing::{Route, Router};
use crate::server::Server;
use tempfile::TempDir;

/// 2 KiB of compressible text, a small text file, a PNG, and a
/// precompressed sibling for `big.css`.
fn fixture() -> TempDir {
    let big = "body { color: red; }\n".repeat(100);
    site(&[
        ("big.txt", big.as_str()),
        ("big.css", big.as_str()),
        ("big.css.br", "brotli bytes"),
        ("small.txt", "tiny"),
        ("image.png", big.as_str()),
    ])
}

fn server(dir: &TempDir, policy: Option<CompressionPolicy>) -> Server {
    let mut builder = Server::builder()
        .address("127.0.0.1:0")
        .document_root(path_str(dir.path()));
    if let Some(policy) = policy {
        builder = builder.compression_policy(policy);
    }
    builder.build().expect("server")
}

fn get(
    server: &Server,
    path: &str,
    accept: &str,
) -> crate::response::Response {
    respond(
        server,
        &request("GET", path, &[("Accept-Encoding", accept)]),
    )
}

#[test]
fn accept_encoding_honours_quality_values() {
    assert!(accepts("gzip, br", "br"));
    assert!(accepts("GZIP;q=0.5", "gzip"));
    assert!(!accepts("gzip;q=0", "gzip"));
    assert!(!accepts("gzip;q=0.0, br", "gzip"));
    assert!(accepts("*", "zstd"));
    assert!(!accepts("*;q=0", "zstd"));
    assert!(!accepts("gzip;q=0, *", "gzip"));
    assert!(!accepts("deflate", "gzip"));
    assert!(!accepts("", "gzip"));
}

#[test]
fn without_policy_precompressed_siblings_respect_q_zero() {
    let dir = fixture();
    let server = server(&dir, None);
    let served = get(&server, "/big.css", "br");
    assert_eq!(header(&served, "content-encoding"), Some("br"));
    let refused = get(&server, "/big.css", "br;q=0");
    assert_eq!(header(&refused, "content-encoding"), None);
}

#[test]
fn policy_prefers_precompressed_sibling() {
    let dir = fixture();
    let server = server(&dir, Some(CompressionPolicy::default()));
    let before = stats().precompressed;
    let response = get(&server, "/big.css", "gzip, br");
    assert_eq!(header(&response, "content-encoding"), Some("br"));
    assert_eq!(response.body, b"brotli bytes");
    assert!(stats().precompressed > before);
}

#[test]
fn ineligible_responses_are_not_compressed() {
    let dir = fixture();
    let server = server(&dir, Some(CompressionPolicy::default()));
    let before = stats();
    // Below the 1024-byte threshold.
    assert_eq!(
        header(&get(&server, "/small.txt", "gzip"), "content-encoding"),
        None
    );
    // Already-compressed format.
    assert_eq!(
        header(&get(&server, "/image.png", "gzip"), "content-encoding"),
        None
    );
    // Client accepts nothing we offer.
    assert_eq!(
        header(
            &get(&server, "/big.txt", "deflate"),
            "content-encoding"
        ),
        None
    );
    let after = stats();
    assert!(after.skipped_size > before.skipped_size);
    assert!(after.skipped_type > before.skipped_type);
    assert!(after.skipped_client > before.skipped_client);
}

#[test]
fn disabled_policy_serves_plain_file() {
    let dir = fixture();
    let policy = CompressionPolicy {
        enabled: false,
        ..CompressionPolicy::default()
    };
    let server = server(&dir, Some(policy));
    let before = stats().skipped_disabled;
    let response = get(&server, "/big.css", "br");
    assert_eq!(header(&response, "content-encoding"), None);
    assert!(response.body.starts_with(b"body {"));
    assert!(stats().skipped_disabled > before);
}

#[test]
fn route_can_switch_compression_off() {
    let dir = fixture();
    let router = Router::new(vec![
        Route::new("plain", "/plain", dir.path()).compression(false),
        Route::new("site", "/", dir.path()),
    ])
    .expect("router");
    let server = Server::builder()
        .address("127.0.0.1:0")
        .document_root(path_str(dir.path()))
        .router(router)
        .build()
        .expect("server");
    let plain = get(&server, "/plain/big.css", "br");
    assert_eq!(header(&plain, "content-encoding"), None);
    let head = respond(
        &server,
        &request(
            "HEAD",
            "/plain/big.css",
            &[("Accept-Encoding", "br")],
        ),
    );
    assert_eq!(header(&head, "content-encoding"), None);
    let site = get(&server, "/big.css", "br");
    assert_eq!(header(&site, "content-encoding"), Some("br"));
}

#[cfg(feature = "compression")]
#[test]
fn dynamic_gzip_compresses_eligible_files() {
    use std::io::Read;
    let dir = fixture();
    let server = server(&dir, Some(CompressionPolicy::default()));
    let before = stats().dynamic;
    let response = get(&server, "/big.txt", "gzip");
    assert_eq!(header(&response, "content-encoding"), Some("gzip"));
    assert_eq!(header(&response, "vary"), Some("Accept-Encoding"));
    let mut plain = String::new();
    let _ = flate2::read::GzDecoder::new(response.body.as_slice())
        .read_to_string(&mut plain)
        .expect("valid gzip");
    assert_eq!(plain, "body { color: red; }\n".repeat(100));
    assert!(stats().dynamic > before);
    // A range request is served from the identity body.
    let ranged = respond(
        &server,
        &request(
            "GET",
            "/big.txt",
            &[("Accept-Encoding", "gzip"), ("Range", "bytes=0-3")],
        ),
    );
    assert_eq!(
        (ranged.status_code, header(&ranged, "content-encoding")),
        (206, None)
    );
}

#[test]
fn metrics_endpoint_reports_compression_outcomes() {
    let response = crate::server::generate_metrics_response();
    let lines = String::from_utf8(response.body).expect("utf-8");
    for outcome in [
        "precompressed",
        "dynamic",
        "disabled",
        "client",
        "type",
        "size",
        "no_variant",
    ] {
        assert!(
            lines.contains(&format!(
                "http_handle_compression_total{{outcome=\"{outcome}\"}}"
            )),
            "{outcome}"
        );
    }
}
