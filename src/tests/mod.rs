// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Crate-internal tests for the request policies, driven through the
//! same `build_response_for_request` entry point every frontend uses.

use crate::request::Request;
use crate::response::Response;
use crate::server::{Server, build_response_for_request};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

mod compression;
mod conditional;
mod routing;
mod waf;

/// A request with the given method, path and headers.
pub(crate) fn request(
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> Request {
    Request {
        method: method.to_string(),
        path: path.to_string(),
        version: "HTTP/1.1".to_string(),
        headers: headers
            .iter()
            .map(|(n, v)| (n.to_ascii_lowercase(), (*v).to_string()))
            .collect(),
    }
}

/// A temporary directory holding `files` (relative path, contents).
pub(crate) fn site(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    for (path, contents) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(full, contents).expect("write");
    }
    dir
}

pub(crate) fn path_str(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}

pub(crate) fn respond(server: &Server, request: &Request) -> Response {
    build_response_for_request(server, request)
}

pub(crate) fn header<'a>(
    response: &'a Response,
    name: &str,
) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}
