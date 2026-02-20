// src/lib.rs

#![doc = include_str!("../README.md")]
#![doc(
    html_favicon_url = "https://kura.pro/http-handle/images/favicon.ico",
    html_logo_url = "https://kura.pro/http-handle/images/logos/http-handle.svg",
    html_root_url = "https://docs.rs/http-handle"
)]
#![cfg_attr(miri, allow(deprecated_in_future))]
#![crate_name = "http_handle"]
#![crate_type = "lib"]

//! # HTTP Handle
//!
//! The `http-handle` is a robust Rust library designed for serving static websites. It provides a simple yet efficient HTTP server implementation with features like request parsing, response generation, and basic security measures. The library is not intended to be a full-fledged web server but rather a lightweight solution for serving static files over HTTP for development and testing purposes.
//!
//! ## Modules
//! - [`server`]: Contains the core `Server` struct and logic for managing HTTP connections.
//! - [`request`]: Handles incoming HTTP requests, parsing and validation.
//! - [`response`]: Provides utilities for crafting HTTP responses.
//! - [`error`]: Defines errors related to the server's operation.
//!

/// The `server` module contains the core `Server` struct and associated methods for starting
/// and managing the HTTP server.
pub mod server;

/// The `request` module is responsible for parsing and validating incoming HTTP requests.
pub mod request;

/// The `response` module provides tools and utilities for crafting HTTP responses.
pub mod response;

/// The `error` module defines various errors that can occur during server operation, including
/// those related to connections and malformed requests.
pub mod error;

/// Language detection primitives with runtime custom patterns.
pub mod language;

/// Async helpers for hardened blocking task execution.
pub mod async_runtime;

#[cfg(feature = "async")]
/// Async Tokio-based server entrypoints.
pub mod async_server;

#[cfg(feature = "batch")]
/// Batch processing APIs.
pub mod batch;

#[cfg(feature = "streaming")]
/// Chunked streaming APIs.
pub mod streaming;

#[cfg(feature = "optimized")]
/// Zero-cost optimized lookups.
pub mod optimized;

#[cfg(feature = "http2")]
/// HTTP/2 server entrypoints.
pub mod http2_server;

#[cfg(feature = "high-perf")]
/// High-performance async-first server with backpressure.
pub mod perf_server;

#[cfg(feature = "http3-profile")]
/// HTTP/3 production profile and ALPN fallback helpers.
pub mod http3_profile;

#[cfg(feature = "distributed-rate-limit")]
/// Distributed rate-limiting backends and adapters.
pub mod distributed_rate_limit;

#[cfg(feature = "multi-tenant")]
/// Multi-tenant config isolation and secret provider integration helpers.
pub mod tenant_isolation;

#[cfg(feature = "autotune")]
/// Runtime host-profile auto-tuning helpers.
pub mod runtime_autotune;

/// Protocol state-machine helpers for fuzzing and conformance testing.
pub mod protocol_state;

#[cfg(feature = "enterprise")]
/// Enterprise configuration, auth, and policy helpers.
pub mod enterprise;

/// Observability helpers.
pub mod observability;

pub use error::ServerError;
pub use language::{Language, LanguageDetector};
pub use server::{
    ConnectionPool, Server, ServerBuilder, ShutdownSignal, ThreadPool,
};

#[cfg(all(test, miri))]
mod miri_smoke {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn response_serialization_smoke() {
        let mut response =
            response::Response::new(200, "OK", b"miri".to_vec());
        response.add_header("Content-Type", "text/plain");
        let mut out = Cursor::new(Vec::<u8>::new());
        response.send(&mut out).expect("send");
        assert!(!out.get_ref().is_empty());
    }

    #[test]
    fn connection_pool_smoke() {
        let pool = ConnectionPool::new(1);
        let first = pool.acquire().expect("acquire");
        assert_eq!(pool.active_count(), 1);
        assert!(pool.acquire().is_err());
        drop(first);
        assert_eq!(pool.active_count(), 0);
    }
}
