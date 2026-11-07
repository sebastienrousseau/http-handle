// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Response compression policy.
//!
//! Without a policy the server keeps its long-standing behaviour: it
//! serves a precompressed sibling (`.br`, `.zst`, `.gz`) of a file when
//! the client accepts that encoding. Setting a [`CompressionPolicy`]
//! with [`ServerBuilder::compression_policy`](crate::ServerBuilder::compression_policy)
//! adds the checks below, in order; the first that applies decides:
//!
//! 1. the policy, or the matched route ([`Route::compression`](crate::routing::Route::compression)),
//!    disables compression: the plain file is served;
//! 2. the client accepts none of `br`, `zstd` or `gzip` (`q=0` counts as
//!    not accepted);
//! 3. the content type is not in [`CompressionPolicy::compressible_types`],
//!    which leaves already-compressed formats such as PNG or ZIP alone;
//! 4. the file is smaller than [`CompressionPolicy::min_size_bytes`];
//! 5. an accepted precompressed sibling exists: it is served;
//! 6. with the `compression` feature and [`CompressionPolicy::dynamic_gzip`],
//!    and no `Range` header, the body is gzip-compressed on the fly.
//!
//! Each decision is counted; [`stats`] returns the counters and the
//! `/metrics` endpoint prints them.
//!
//! # Examples
//!
//! ```rust
//! use http_handle::compression::CompressionPolicy;
//! use http_handle::Server;
//!
//! let server = Server::builder()
//!     .address("127.0.0.1:0")
//!     .document_root(".")
//!     .compression_policy(CompressionPolicy {
//!         min_size_bytes: 512,
//!         ..CompressionPolicy::default()
//!     })
//!     .build()
//!     .expect("server");
//! assert!(server.compression_policy().is_some());
//! ```

use crate::request::Request;
use crate::server::negotiate_precompressed_variant;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// When and how responses are compressed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressionPolicy {
    /// `false` serves every file uncompressed. Defaults to `true`.
    pub enabled: bool,
    /// Files smaller than this many bytes are served uncompressed.
    /// Defaults to 1024.
    pub min_size_bytes: u64,
    /// Compress with gzip on the fly when no precompressed sibling
    /// exists. Needs the `compression` feature; ignored without it.
    /// Defaults to `true`.
    pub dynamic_gzip: bool,
    /// Content-type prefixes that may be compressed.
    pub compressible_types: Vec<String>,
}

impl Default for CompressionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            min_size_bytes: 1024,
            dynamic_gzip: true,
            compressible_types: [
                "text/",
                "application/javascript",
                "application/json",
                "application/manifest+json",
                "application/typescript",
                "application/wasm",
                "application/x-yaml",
                "application/toml",
                "application/xml",
                "image/svg+xml",
            ]
            .map(String::from)
            .to_vec(),
        }
    }
}

/// Why a response was or was not compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Precompressed,
    Dynamic,
    Disabled,
    ClientDeclined,
    Type,
    Size,
    NoVariant,
}

const OUTCOMES: usize = 7;
static COUNTERS: [AtomicUsize; OUTCOMES] =
    [const { AtomicUsize::new(0) }; OUTCOMES];

impl Outcome {
    const ALL: [Outcome; OUTCOMES] = [
        Outcome::Precompressed,
        Outcome::Dynamic,
        Outcome::Disabled,
        Outcome::ClientDeclined,
        Outcome::Type,
        Outcome::Size,
        Outcome::NoVariant,
    ];

    fn label(self) -> &'static str {
        match self {
            Outcome::Precompressed => "precompressed",
            Outcome::Dynamic => "dynamic",
            Outcome::Disabled => "disabled",
            Outcome::ClientDeclined => "client",
            Outcome::Type => "type",
            Outcome::Size => "size",
            Outcome::NoVariant => "no_variant",
        }
    }

    pub(crate) fn record(self) {
        let _ = COUNTERS[self as usize].fetch_add(1, Ordering::Relaxed);
    }
}

/// Counts of compression decisions since the process started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompressionStats {
    /// Served a precompressed sibling.
    pub precompressed: usize,
    /// Compressed with gzip on the fly.
    pub dynamic: usize,
    /// Skipped: disabled by the policy or the route.
    pub skipped_disabled: usize,
    /// Skipped: the client accepts no supported encoding.
    pub skipped_client: usize,
    /// Skipped: content type not compressible.
    pub skipped_type: usize,
    /// Skipped: below the minimum size.
    pub skipped_size: usize,
    /// Skipped: no precompressed sibling and no dynamic compression.
    pub skipped_no_variant: usize,
}

/// Returns the compression decision counters.
pub fn stats() -> CompressionStats {
    let get = |o: Outcome| COUNTERS[o as usize].load(Ordering::Relaxed);
    CompressionStats {
        precompressed: get(Outcome::Precompressed),
        dynamic: get(Outcome::Dynamic),
        skipped_disabled: get(Outcome::Disabled),
        skipped_client: get(Outcome::ClientDeclined),
        skipped_type: get(Outcome::Type),
        skipped_size: get(Outcome::Size),
        skipped_no_variant: get(Outcome::NoVariant),
    }
}

/// Prometheus text lines for the `/metrics` endpoint.
pub(crate) fn metrics_lines() -> String {
    Outcome::ALL
        .iter()
        .map(|o| {
            format!(
                "http_handle_compression_total{{outcome=\"{}\"}} {}\n",
                o.label(),
                COUNTERS[*o as usize].load(Ordering::Relaxed)
            )
        })
        .collect()
}

/// Whether the `Accept-Encoding` value accepts `coding` with a non-zero
/// quality. An explicit entry for the coding wins over `*`.
pub(crate) fn accepts(accept_encoding: &str, coding: &str) -> bool {
    let mut wildcard = None;
    for entry in accept_encoding.split(',') {
        let mut fields = entry.split(';');
        let name = fields.next().unwrap_or_default().trim();
        let quality = fields
            .filter_map(|p| p.trim().strip_prefix("q="))
            .find_map(|q| q.trim().parse::<f32>().ok())
            .unwrap_or(1.0);
        if name.eq_ignore_ascii_case(coding) {
            return quality > 0.0;
        }
        if name == "*" {
            wildcard = Some(quality > 0.0);
        }
    }
    wildcard.unwrap_or(false)
}

/// The checks that run before a variant is looked for: `None` means
/// compression may go ahead.
pub(crate) fn precheck(
    policy: &CompressionPolicy,
    accept_encoding: Option<&str>,
    content_type: &str,
    len: u64,
) -> Option<Outcome> {
    let accepted = accept_encoding.is_some_and(|accept| {
        ["br", "zstd", "zst", "gzip"]
            .iter()
            .any(|coding| accepts(accept, coding))
    });
    if !policy.enabled {
        Some(Outcome::Disabled)
    } else if !accepted {
        Some(Outcome::ClientDeclined)
    } else if !policy
        .compressible_types
        .iter()
        .any(|prefix| content_type.starts_with(prefix.as_str()))
    {
        Some(Outcome::Type)
    } else if len < policy.min_size_bytes {
        Some(Outcome::Size)
    } else {
        None
    }
}

/// Whether to gzip the body on the fly.
pub(crate) fn wants_dynamic_gzip(
    policy: &CompressionPolicy,
    accept_encoding: Option<&str>,
    has_range: bool,
) -> bool {
    cfg!(feature = "compression")
        && policy.dynamic_gzip
        && !has_range
        && accept_encoding.is_some_and(|accept| accepts(accept, "gzip"))
}

/// Gzip-compresses `body`.
#[cfg(feature = "compression")]
pub(crate) fn gzip(body: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;
    let mut encoder =
        GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(body)?;
    encoder.finish()
}

/// Without the `compression` feature dynamic gzip is never chosen.
#[cfg(not(feature = "compression"))]
pub(crate) fn gzip(_body: &[u8]) -> std::io::Result<Vec<u8>> {
    Err(std::io::Error::other(
        "built without the compression feature",
    ))
}

/// Which compression rules apply to one request.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Mode<'a> {
    /// No policy: precompressed siblings only, nothing counted.
    Legacy,
    /// Switched off by the matched route.
    Off,
    /// The server's policy.
    Policy(&'a CompressionPolicy),
}

impl<'a> Mode<'a> {
    /// The mode for a server policy and a route's override.
    pub(crate) fn new(
        policy: Option<&'a CompressionPolicy>,
        route_override: Option<bool>,
    ) -> Self {
        match (route_override, policy) {
            (Some(false), _) => Mode::Off,
            (_, Some(policy)) => Mode::Policy(policy),
            (_, None) => Mode::Legacy,
        }
    }
}

/// The representation chosen for a file.
#[derive(Debug)]
pub(crate) enum Encoding {
    /// The file as stored.
    Identity,
    /// A precompressed sibling and its `Content-Encoding`.
    Precompressed(PathBuf, &'static str),
    /// The file gzip-compressed on the fly.
    Gzip,
}

impl Encoding {
    /// The `Content-Encoding` value, if any.
    pub(crate) fn header(&self) -> Option<&'static str> {
        match self {
            Encoding::Identity => None,
            Encoding::Precompressed(_, coding) => Some(coding),
            Encoding::Gzip => Some("gzip"),
        }
    }
}

/// Chooses how to encode the file at `path` (of `len` bytes) for
/// `request`, and counts the decision under a policy.
pub(crate) fn select(
    mode: Mode<'_>,
    request: &Request,
    path: &Path,
    canonical_root: &Path,
    content_type: &str,
    len: u64,
) -> Encoding {
    let accept = request.header("accept-encoding");
    let policy = match mode {
        Mode::Off => {
            Outcome::Disabled.record();
            return Encoding::Identity;
        }
        Mode::Legacy => None,
        Mode::Policy(policy) => Some(policy),
    };
    if let Some(skip) =
        policy.and_then(|p| precheck(p, accept, content_type, len))
    {
        skip.record();
        return Encoding::Identity;
    }
    if let (sibling, Some(coding)) =
        negotiate_precompressed_variant(path, accept, canonical_root)
    {
        if policy.is_some() {
            Outcome::Precompressed.record();
        }
        return Encoding::Precompressed(sibling, coding);
    }
    let Some(policy) = policy else {
        return Encoding::Identity;
    };
    if wants_dynamic_gzip(
        policy,
        accept,
        request.header("range").is_some(),
    ) {
        Outcome::Dynamic.record();
        return Encoding::Gzip;
    }
    Outcome::NoVariant.record();
    Encoding::Identity
}
