#![forbid(unsafe_code)]
// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

// src/lib.rs
#![doc = include_str!("../README.md")]
#![doc(
    html_favicon_url = "https://kura.pro/http-handle/images/favicon.ico",
    html_logo_url = "https://kura.pro/http-handle/images/logos/http-handle.svg",
    html_root_url = "https://docs.rs/http-handle"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(miri, allow(deprecated_in_future))]
#![crate_name = "http_handle"]
#![crate_type = "lib"]

pub mod server;

pub mod request;

pub mod response;

pub mod error;

pub mod language;

pub mod async_runtime;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub mod async_server;

#[cfg(feature = "batch")]
#[cfg_attr(docsrs, doc(cfg(feature = "batch")))]
pub mod batch;

#[cfg(feature = "streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "streaming")))]
pub mod streaming;

#[cfg(feature = "optimized")]
#[cfg_attr(docsrs, doc(cfg(feature = "optimized")))]
pub mod optimized;

#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
pub mod http2_server;

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
pub mod perf_server;

#[cfg(feature = "http3-profile")]
#[cfg_attr(docsrs, doc(cfg(feature = "http3-profile")))]
pub mod http3_profile;

#[cfg(feature = "distributed-rate-limit")]
#[cfg_attr(docsrs, doc(cfg(feature = "distributed-rate-limit")))]
pub mod distributed_rate_limit;

#[cfg(feature = "multi-tenant")]
#[cfg_attr(docsrs, doc(cfg(feature = "multi-tenant")))]
pub mod tenant_isolation;

#[cfg(feature = "autotune")]
#[cfg_attr(docsrs, doc(cfg(feature = "autotune")))]
pub mod runtime_autotune;

pub mod protocol_state;

#[cfg(feature = "enterprise")]
#[cfg_attr(docsrs, doc(cfg(feature = "enterprise")))]
pub mod enterprise;

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
