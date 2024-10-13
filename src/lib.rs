// src/lib.rs

#![doc = include_str!("../README.md")]
#![doc(
    html_favicon_url = "https://kura.pro/http-handle/images/favicon.ico",
    html_logo_url = "https://kura.pro/http-handle/images/logos/http-handle.svg",
    html_root_url = "https://docs.rs/http-handle"
)]
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

pub use error::ServerError;
pub use server::Server;
