// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2023 - 2026 HTTP Handle

//! Optional request policies a [`Server`](crate::Server) applies to
//! every request, whichever frontend parsed it.

use crate::routing::Router;
use serde::{Deserialize, Serialize};

/// The optional policies attached to a server. Each is `None` until
/// configured through [`ServerBuilder`](crate::ServerBuilder).
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize,
)]
pub(crate) struct RequestPolicies {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) router: Option<Router>,
}

impl RequestPolicies {
    /// Whether no policy is configured, so the plain static path and
    /// its fast paths apply unchanged.
    pub(crate) fn is_empty(&self) -> bool {
        self.router.is_none()
    }
}
