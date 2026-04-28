// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Unified runner: exercises core API and all enabled feature modules.
//!
//! Run: `cargo run --example full --all-features`

#[path = "support.rs"]
mod support;

use http_handle::response::Response;
use http_handle::{Language, LanguageDetector, Server, ServerError};
use std::io::Cursor;
use std::time::Duration;

fn main() {
    support::header("http-handle -- full");

    support::task_with_output(
        "Core: ServerBuilder + policy stack",
        || {
            let server = Server::builder()
                .address("127.0.0.1:8080")
                .document_root("./public")
                .enable_cors()
                .cors_origins(vec!["https://example.com".into()])
                .custom_header("X-Content-Type-Options", "nosniff")
                .request_timeout(Duration::from_secs(15))
                .connection_timeout(Duration::from_secs(30))
                .rate_limit_per_minute(120)
                .static_cache_ttl_secs(300)
                .build()
                .expect("build");
            vec![
                format!("address = {}", server.address()),
                format!(
                    "root    = {}",
                    server.document_root().display()
                ),
            ]
        },
    );

    support::task_with_output("Core: language detection", || {
        let detector = LanguageDetector::new()
            .with_custom_pattern(Language::Rust, r"\bcrate::\w+")
            .expect("regex");
        let language = detector.detect("use crate::server::Server;");
        vec![format!("detected = {}", language.as_str())]
    });

    support::task_with_output("Core: response serialisation", || {
        let mut response = Response::new(200, "OK", b"hello".to_vec());
        response.add_header("Content-Type", "text/plain");
        let mut wire = Cursor::new(Vec::<u8>::new());
        response.send(&mut wire).expect("send");
        vec![format!("bytes = {}", wire.get_ref().len())]
    });

    support::task_with_output("Core: error construction", || {
        let error = ServerError::invalid_request("missing host header");
        vec![format!("err = {error}")]
    });

    feature_section();

    support::summary(4 + feature_section_count());
}

fn feature_section() {
    demo_batch();
    demo_streaming();
    demo_optimized();
    demo_observability();
    demo_async_runtime();
    demo_http2_marker();
    demo_high_perf_marker();
}

const FEATURE_SLOTS: usize = 7;
fn feature_section_count() -> usize {
    FEATURE_SLOTS
}

#[cfg(feature = "batch")]
fn demo_batch() {
    use http_handle::batch::{BatchRequest, process_batch};
    use std::path::PathBuf;
    support::task_with_output("[feature=batch] process_batch", || {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join("a.txt"), b"alpha").expect("a");
        std::fs::write(root.path().join("b.txt"), b"beta").expect("b");
        let requests = vec![
            BatchRequest {
                relative_path: PathBuf::from("a.txt"),
            },
            BatchRequest {
                relative_path: PathBuf::from("b.txt"),
            },
        ];
        let results = process_batch(root.path(), &requests, 2);
        vec![format!("processed = {} files", results.len())]
    });
}

#[cfg(not(feature = "batch"))]
fn demo_batch() {
    support::task("[feature=batch] disabled", || {});
}

#[cfg(feature = "streaming")]
fn demo_streaming() {
    use http_handle::streaming::ChunkStream;
    support::task_with_output(
        "[feature=streaming] ChunkStream",
        || {
            let root = tempfile::tempdir().expect("tempdir");
            let file = root.path().join("data.txt");
            std::fs::write(&file, b"abcdefgh").expect("seed");
            let chunks: Result<Vec<Vec<u8>>, _> =
                ChunkStream::from_file(&file, 3)
                    .expect("open")
                    .collect();
            vec![format!("chunks = {}", chunks.expect("collect").len())]
        },
    );
}

#[cfg(not(feature = "streaming"))]
fn demo_streaming() {
    support::task("[feature=streaming] disabled", || {});
}

#[cfg(feature = "optimized")]
fn demo_optimized() {
    use http_handle::optimized::{
        LanguageSet, const_content_type_from_ext, detect_language_fast,
    };
    support::task_with_output(
        "[feature=optimized] fast helpers",
        || {
            let mut set = LanguageSet::new();
            set.insert(Language::Rust);
            set.insert(Language::Python);
            let lang = detect_language_fast("fn main() {}");
            let mime = const_content_type_from_ext("wasm");
            vec![
                format!("unique  = {}", set.as_slice().len()),
                format!("lang    = {}", lang.as_str()),
                format!("mime    = {mime}"),
            ]
        },
    );
}

#[cfg(not(feature = "optimized"))]
fn demo_optimized() {
    support::task("[feature=optimized] disabled", || {});
}

fn demo_observability() {
    support::task("[feature=observability] init_tracing", || {
        http_handle::observability::init_tracing();
    });
}

#[cfg(feature = "async")]
fn demo_async_runtime() {
    support::task_with_output("[feature=async] run_blocking", || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let value = runtime
            .block_on(async {
                http_handle::async_runtime::run_blocking(|| {
                    Ok::<_, ServerError>(42)
                })
                .await
            })
            .expect("run_blocking");
        vec![format!("answer = {value}")]
    });
}

#[cfg(not(feature = "async"))]
fn demo_async_runtime() {
    support::task("[feature=async] disabled", || {});
}

#[cfg(feature = "http2")]
fn demo_http2_marker() {
    support::task(
        "[feature=http2] enabled (see `http2` example)",
        || {},
    );
}

#[cfg(not(feature = "http2"))]
fn demo_http2_marker() {
    support::task("[feature=http2] disabled", || {});
}

#[cfg(feature = "high-perf")]
fn demo_high_perf_marker() {
    support::task(
        "[feature=high-perf] enabled (see `perf` / `multi` examples)",
        || {},
    );
}

#[cfg(not(feature = "high-perf"))]
fn demo_high_perf_marker() {
    support::task("[feature=high-perf] disabled", || {});
}
