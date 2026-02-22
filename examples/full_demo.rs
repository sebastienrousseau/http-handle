// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Unified capability runner example.
//!
//! Run this example to exercise core functionality and, when enabled,
//! feature-specific modules in one place.

use http_handle::response::Response;
use http_handle::{Language, LanguageDetector, Server, ServerError};
use std::io::Cursor;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_core_demo()?;
    run_feature_demos()?;
    println!("All enabled full_demo demos completed.");
    Ok(())
}

fn run_core_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("[core] builder + policy configuration");
    let server = Server::builder()
        .address("127.0.0.1:8080")
        .document_root("./public")
        .enable_cors()
        .cors_origins(vec!["https://example.com".to_string()])
        .custom_header("X-Content-Type-Options", "nosniff")
        .request_timeout(Duration::from_secs(15))
        .connection_timeout(Duration::from_secs(30))
        .rate_limit_per_minute(120)
        .static_cache_ttl_secs(300)
        .build()?;

    println!("  address={}", server.address());
    println!("  root={}", server.document_root().display());

    println!("[core] language detection");
    let detector = LanguageDetector::new()
        .with_custom_pattern(Language::Rust, r"\\bcrate::\\w+")?;
    let language = detector.detect("use crate::server::Server;");
    println!("  detected={}", language.as_str());

    println!("[core] response serialization");
    let mut response = Response::new(200, "OK", b"hello".to_vec());
    response.add_header("Content-Type", "text/plain");
    let mut wire = Cursor::new(Vec::<u8>::new());
    response.send(&mut wire)?;
    println!("  serialized_bytes={}", wire.get_ref().len());

    println!("[core] error construction");
    let error = ServerError::invalid_request("missing host header");
    println!("  sample_error={error}");

    Ok(())
}

fn run_feature_demos() -> Result<(), Box<dyn std::error::Error>> {
    demo_batch()?;
    demo_streaming()?;
    demo_optimized()?;
    demo_observability();
    demo_async_runtime()?;
    demo_http2_marker()?;
    Ok(())
}

#[cfg(feature = "batch")]
fn demo_batch() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::batch::{BatchRequest, process_batch};
    use std::path::PathBuf;

    println!("[feature=batch] process_batch");
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.txt"), b"alpha")?;
    std::fs::write(root.path().join("b.txt"), b"beta")?;

    let requests = vec![
        BatchRequest {
            relative_path: PathBuf::from("a.txt"),
        },
        BatchRequest {
            relative_path: PathBuf::from("b.txt"),
        },
    ];

    let results = process_batch(root.path(), &requests, 2);
    println!("  files_processed={}", results.len());
    Ok(())
}

#[cfg(not(feature = "batch"))]
fn demo_batch() -> Result<(), Box<dyn std::error::Error>> {
    println!("[feature=batch] disabled");
    Ok(())
}

#[cfg(feature = "streaming")]
fn demo_streaming() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::streaming::ChunkStream;

    println!("[feature=streaming] chunk stream");
    let root = tempfile::tempdir()?;
    let file = root.path().join("data.txt");
    std::fs::write(&file, b"abcdefgh")?;

    let chunks: Result<Vec<Vec<u8>>, _> =
        ChunkStream::from_file(&file, 3)?.collect();
    println!("  chunks={}", chunks?.len());
    Ok(())
}

#[cfg(not(feature = "streaming"))]
fn demo_streaming() -> Result<(), Box<dyn std::error::Error>> {
    println!("[feature=streaming] disabled");
    Ok(())
}

#[cfg(feature = "optimized")]
fn demo_optimized() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::optimized::{
        LanguageSet, const_content_type_from_ext, detect_language_fast,
    };

    println!("[feature=optimized] fast helpers");
    let mut set = LanguageSet::new();
    set.insert(Language::Rust);
    set.insert(Language::Python);
    let lang = detect_language_fast("fn main() {}");
    let mime = const_content_type_from_ext("wasm");

    println!("  unique_languages={}", set.as_slice().len());
    println!("  lang={} mime={}", lang.as_str(), mime);
    Ok(())
}

#[cfg(not(feature = "optimized"))]
fn demo_optimized() -> Result<(), Box<dyn std::error::Error>> {
    println!("[feature=optimized] disabled");
    Ok(())
}

fn demo_observability() {
    println!("[feature=observability] init tracing");
    http_handle::observability::init_tracing();
    #[cfg(feature = "observability")]
    {
        tracing::info!(target: "http_handle::example", "observability active");
    }
}

#[cfg(feature = "async")]
fn demo_async_runtime() -> Result<(), Box<dyn std::error::Error>> {
    println!("[feature=async] run_blocking");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let value = runtime.block_on(async {
        http_handle::async_runtime::run_blocking(|| {
            Ok::<_, ServerError>(42)
        })
        .await
    })?;
    println!("  run_blocking_value={value}");
    Ok(())
}

#[cfg(not(feature = "async"))]
fn demo_async_runtime() -> Result<(), Box<dyn std::error::Error>> {
    println!("[feature=async] disabled");
    Ok(())
}

#[cfg(feature = "http2")]
fn demo_http2_marker() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[feature=http2] module enabled (use feature_http2_server example for full request flow)"
    );
    Ok(())
}

#[cfg(not(feature = "http2"))]
fn demo_http2_marker() -> Result<(), Box<dyn std::error::Error>> {
    println!("[feature=http2] disabled");
    Ok(())
}
