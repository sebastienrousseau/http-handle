// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Async Tokio server entrypoints.
//!
//! This module provides the async accept loop that bridges into the existing request
//! handling stack.

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
use crate::async_runtime::run_blocking;
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
use crate::error::ServerError;
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
use crate::server::Server;

/// Starts an async accept loop backed by Tokio.
///
/// Each accepted connection is converted to a standard stream and served via the
/// existing synchronous connection handler on Tokio's blocking pool.
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
///
/// # Examples
///
/// ```rust,no_run
/// use http_handle::async_server::start_async;
/// use http_handle::Server;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let server = Server::new("127.0.0.1:8080", ".");
/// let _ = start_async(server).await;
/// # }
/// ```
///
/// # Errors
///
/// Returns an error when binding or accept fails.
///
/// # Panics
///
/// This function does not panic.
pub async fn start_async(server: Server) -> Result<(), ServerError> {
    let listener = tokio::net::TcpListener::bind(server.address())
        .await
        .map_err(ServerError::from)?;
    loop {
        let (stream, _) =
            listener.accept().await.map_err(ServerError::from)?;
        let server_clone = server.clone();
        let std_stream =
            stream.into_std().map_err(ServerError::from)?;
        drop(tokio::spawn(async move {
            let _ = run_blocking(move || {
                crate::server::handle_connection(
                    std_stream,
                    &server_clone,
                )
            })
            .await;
        }));
    }
}

#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use tempfile::TempDir;
    use tokio::time::{Duration, sleep};

    fn free_addr() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);
        addr.to_string()
    }

    #[tokio::test]
    async fn async_server_accepts_connections() {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello")
            .expect("write");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("404 page");

        let addr = free_addr();
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let task = tokio::spawn(start_async(server));
        sleep(Duration::from_millis(40)).await;

        let mut stream = TcpStream::connect(&addr).expect("connect");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("write");
        sleep(Duration::from_millis(80)).await;

        task.abort();
    }
}
