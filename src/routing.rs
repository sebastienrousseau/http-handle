// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Layer-7 routing: pick a document root by host, path prefix, method
//! and headers.
//!
//! A [`Router`] holds validated [`Route`]s. Each route mounts one
//! document root under a path prefix; a request that matches is served
//! from that root with the prefix removed. When a server has a router,
//! a request that matches no route gets a plain `404 Not Found`, never
//! the server's own document root, so routing is deterministic. Add a
//! route with the prefix `/` as an explicit fallback.
//!
//! Routes are tried from the highest [`Route::priority`] down. Among
//! routes of equal priority the more specific one wins: a route with a
//! host before one without, then the longer path prefix, then the route
//! with more header constraints, then one with a method constraint. Two
//! routes with the same priority and the same constraints are rejected
//! as ambiguous by [`Router::new`].
//!
//! # Examples
//!
//! ```rust
//! use http_handle::routing::{Route, Router};
//! use http_handle::Server;
//!
//! let router = Router::new(vec![
//!     Route::new("docs", "/docs", ".").host("docs.example.com"),
//!     Route::new("site", "/", "."),
//! ])
//! .expect("valid routes");
//!
//! let server = Server::builder()
//!     .address("127.0.0.1:0")
//!     .document_root(".")
//!     .router(router)
//!     .build()
//!     .expect("server");
//! assert_eq!(server.router().map(|r| r.routes().len()), Some(2));
//! ```

use crate::request::Request;
use crate::response::Response;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// One routing rule: request constraints and the document root it
/// serves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// Unique name, used in errors and diagnostics.
    pub name: String,
    /// Host the request must name (the `Host` header without its port,
    /// compared case-insensitively). `None` matches any host.
    #[serde(default)]
    pub host: Option<String>,
    /// Path prefix, matched on segment boundaries: `/docs` matches
    /// `/docs` and `/docs/a.html`, not `/docsx`. `/` matches every path.
    pub path_prefix: String,
    /// Allowed methods. Empty allows every method.
    #[serde(default)]
    pub methods: Vec<String>,
    /// Headers that must be present with exactly this value. Names are
    /// compared case-insensitively.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Higher values are tried first. Defaults to 0.
    #[serde(default)]
    pub priority: i32,
    /// Directory served for requests that match this route.
    pub document_root: PathBuf,
}

impl Route {
    /// Creates a route that serves `document_root` under `path_prefix`.
    pub fn new(
        name: &str,
        path_prefix: &str,
        document_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            name: name.to_string(),
            host: None,
            path_prefix: path_prefix.to_string(),
            methods: Vec::new(),
            headers: Vec::new(),
            priority: 0,
            document_root: document_root.into(),
        }
    }

    /// Restricts the route to one host.
    #[must_use]
    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.to_string());
        self
    }

    /// Adds an allowed method.
    #[must_use]
    pub fn method(mut self, method: &str) -> Self {
        self.methods.push(method.to_string());
        self
    }

    /// Requires a header with an exact value.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Sets the priority; higher is tried first.
    #[must_use]
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Whether `request` meets every constraint of this route.
    pub fn matches(&self, request: &Request) -> bool {
        self.host_matches(request)
            && prefix_matches(
                &self.path_prefix,
                path_only(request.path()),
            )
            && (self.methods.is_empty()
                || self.methods.iter().any(|m| m == request.method()))
            && self.headers.iter().all(|(name, value)| {
                request.header(name) == Some(value)
            })
    }

    fn host_matches(&self, request: &Request) -> bool {
        let Some(expected) = self.host.as_deref() else {
            return true;
        };
        request
            .header("host")
            .map(host_without_port)
            .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
    }

    /// Sort key: priority first, then specificity.
    fn rank(&self) -> Reverse<(i32, bool, usize, usize, bool)> {
        Reverse((
            self.priority,
            self.host.is_some(),
            self.path_prefix.len(),
            self.headers.len(),
            !self.methods.is_empty(),
        ))
    }

    /// Normalises names so matching and conflict checks are exact.
    fn normalised(mut self) -> Self {
        self.host = self.host.map(|h| h.to_ascii_lowercase());
        if self.path_prefix.len() > 1 {
            let trimmed = self.path_prefix.trim_end_matches('/');
            self.path_prefix =
                if trimmed.is_empty() { "/" } else { trimmed }.into();
        }
        for method in &mut self.methods {
            method.make_ascii_uppercase();
        }
        self.methods.sort();
        self.methods.dedup();
        for (name, _) in &mut self.headers {
            name.make_ascii_lowercase();
        }
        self.headers.sort();
        self
    }

    /// Everything that decides whether the route matches.
    fn constraints(&self) -> impl PartialEq + '_ {
        (
            self.priority,
            &self.host,
            &self.path_prefix,
            &self.methods,
            &self.headers,
        )
    }
}

/// Why a set of routes was rejected.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RouteError {
    /// A route has an empty name.
    #[error("route name must not be empty")]
    EmptyName,
    /// Two routes share a name.
    #[error("duplicate route name `{0}`")]
    DuplicateName(String),
    /// A path prefix does not start with `/`.
    #[error(
        "route `{route}`: path prefix `{prefix}` must start with `/`"
    )]
    InvalidPathPrefix {
        /// Route name.
        route: String,
        /// The rejected prefix.
        prefix: String,
    },
    /// A method or host is empty, or a method is not an HTTP token.
    #[error("route `{route}`: invalid {field} `{value}`")]
    InvalidConstraint {
        /// Route name.
        route: String,
        /// `host` or `method`.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// The document root is not an existing directory.
    #[error(
        "route `{route}`: document root `{path}` is not a directory"
    )]
    InvalidDocumentRoot {
        /// Route name.
        route: String,
        /// The rejected path.
        path: String,
    },
    /// Two routes have the same priority and constraints, so neither
    /// can be preferred.
    #[error("routes `{first}` and `{second}` are ambiguous")]
    Ambiguous {
        /// First route name.
        first: String,
        /// Second route name.
        second: String,
    },
}

/// A validated, ordered set of routes.
///
/// Serialises as its list of routes; deserialising validates them
/// again, so a stored configuration cannot bypass [`Router::new`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Route>", into = "Vec<Route>")]
pub struct Router {
    routes: Vec<Route>,
    canonical_roots: Vec<PathBuf>,
}

/// Where a routed request is served from.
#[derive(Debug)]
pub(crate) struct RouteTarget<'a> {
    /// The request with the route prefix removed from its path.
    pub(crate) request: Request,
    /// The route's document root.
    pub(crate) document_root: &'a Path,
    /// The route's canonicalised document root.
    pub(crate) canonical_root: &'a Path,
}

impl Router {
    /// Validates `routes` and orders them for matching.
    ///
    /// # Errors
    ///
    /// Returns a [`RouteError`] for an empty or duplicate name, a prefix
    /// that does not start with `/`, an empty host or invalid method, a
    /// document root that is not a directory, or two routes with equal
    /// priority and constraints.
    pub fn new(routes: Vec<Route>) -> Result<Self, RouteError> {
        let mut routes: Vec<Route> =
            routes.into_iter().map(Route::normalised).collect();
        for (index, route) in routes.iter().enumerate() {
            validate_route(route)?;
            if let Some(other) = routes[..index]
                .iter()
                .find(|other| other.name == route.name)
            {
                return Err(RouteError::DuplicateName(
                    other.name.clone(),
                ));
            }
            if let Some(other) = routes[..index].iter().find(|other| {
                other.constraints() == route.constraints()
            }) {
                return Err(RouteError::Ambiguous {
                    first: other.name.clone(),
                    second: route.name.clone(),
                });
            }
        }
        // Stable sort: declaration order breaks the remaining ties.
        routes.sort_by_key(Route::rank);
        let canonical_roots = routes
            .iter()
            .map(canonical_root)
            .collect::<Result<_, _>>()?;
        Ok(Self {
            routes,
            canonical_roots,
        })
    }

    /// The routes in matching order.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// The first route, in matching order, that `request` meets.
    pub fn select(&self, request: &Request) -> Option<&Route> {
        self.routes.iter().find(|route| route.matches(request))
    }

    /// Resolves `request` to the root that serves it.
    pub(crate) fn resolve(
        &self,
        request: &Request,
    ) -> Option<RouteTarget<'_>> {
        let index = self
            .routes
            .iter()
            .position(|route| route.matches(request))?;
        let route = &self.routes[index];
        let mut routed = request.clone();
        routed.path = strip_prefix(&route.path_prefix, request.path());
        Some(RouteTarget {
            request: routed,
            document_root: &route.document_root,
            canonical_root: &self.canonical_roots[index],
        })
    }
}

impl TryFrom<Vec<Route>> for Router {
    type Error = RouteError;

    fn try_from(routes: Vec<Route>) -> Result<Self, Self::Error> {
        Self::new(routes)
    }
}

impl From<Router> for Vec<Route> {
    fn from(router: Router) -> Self {
        router.routes
    }
}

/// The response for a request that matches no route.
pub(crate) fn no_route_response() -> Response {
    let mut response =
        Response::new(404, "NOT FOUND", b"404 Not Found".to_vec());
    response.add_header("Content-Type", "text/plain");
    response
}

fn validate_route(route: &Route) -> Result<(), RouteError> {
    if route.name.is_empty() {
        return Err(RouteError::EmptyName);
    }
    if !route.path_prefix.starts_with('/') {
        return Err(RouteError::InvalidPathPrefix {
            route: route.name.clone(),
            prefix: route.path_prefix.clone(),
        });
    }
    let invalid = |field, value: &str| RouteError::InvalidConstraint {
        route: route.name.clone(),
        field,
        value: value.to_string(),
    };
    if let Some(host) = route.host.as_deref().filter(|h| h.is_empty()) {
        return Err(invalid("host", host));
    }
    if let Some(method) = route.methods.iter().find(|m| !is_token(m)) {
        return Err(invalid("method", method));
    }
    Ok(())
}

fn canonical_root(route: &Route) -> Result<PathBuf, RouteError> {
    fs::canonicalize(&route.document_root)
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(|| RouteError::InvalidDocumentRoot {
            route: route.name.clone(),
            path: route.document_root.display().to_string(),
        })
}

fn is_token(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_uppercase())
}

fn path_only(path: &str) -> &str {
    path.split_once('?').map_or(path, |(path, _)| path)
}

fn prefix_matches(prefix: &str, path: &str) -> bool {
    prefix == "/"
        || path.strip_prefix(prefix).is_some_and(|rest| {
            rest.is_empty() || rest.starts_with('/')
        })
}

/// Removes the route prefix, keeping a leading `/`.
fn strip_prefix(prefix: &str, path: &str) -> String {
    if prefix == "/" {
        return path.to_string();
    }
    let rest = path.strip_prefix(prefix).unwrap_or(path);
    if rest.starts_with('/') {
        rest.to_string()
    } else {
        format!("/{rest}")
    }
}

/// `example.com:8080` to `example.com`, `[::1]:8080` to `[::1]`.
fn host_without_port(host: &str) -> &str {
    let host = host.trim();
    if host.starts_with('[') {
        return host.split_inclusive(']').next().unwrap_or(host);
    }
    match host.rsplit_once(':') {
        Some((name, port))
            if !port.is_empty()
                && port.bytes().all(|b| b.is_ascii_digit()) =>
        {
            name
        }
        _ => host,
    }
}
