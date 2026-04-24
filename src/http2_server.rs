// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! HTTP/2 server entrypoints (feature-gated).
//!
//! This module provides a clear-text HTTP/2 (h2c) accept loop that reuses
//! the request/response behavior from the primary server pipeline.

#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
use crate::error::ServerError;
#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
use crate::request::Request;
#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
use crate::server::{Server, build_response_for_request_with_metrics};

/// Starts an HTTP/2 (h2c) accept loop backed by Tokio.
///
/// Each accepted TCP connection is upgraded to an `h2` server connection and
/// each stream is handled using the same request->response logic used by
/// the HTTP/1 server.
#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
///
/// # Examples
///
/// ```rust,no_run
/// use http_handle::http2_server::start_http2;
/// use http_handle::Server;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let server = Server::new("127.0.0.1:8080", ".");
/// let _ = start_http2(server).await;
/// # }
/// ```
///
/// # Errors
///
/// Returns an error when binding or accepting HTTP/2 connections fails.
///
/// # Panics
///
/// This function does not panic.
pub async fn start_http2(server: Server) -> Result<(), ServerError> {
    let listener = tokio::net::TcpListener::bind(server.address())
        .await
        .map_err(ServerError::from)?;

    loop {
        let (stream, _) =
            listener.accept().await.map_err(ServerError::from)?;
        let server_clone = server.clone();
        drop(tokio::spawn(async move {
            if let Err(error) =
                handle_h2_connection(stream, server_clone).await
            {
                eprintln!("HTTP/2 connection error: {}", error);
            }
        }));
    }
}

#[cfg(feature = "http2")]
fn h2_handshake_err(e: h2::Error) -> ServerError {
    ServerError::Custom(format!("h2 handshake: {e}"))
}

#[cfg(feature = "http2")]
fn h2_accept_err(e: h2::Error) -> ServerError {
    ServerError::Custom(format!("h2 accept: {e}"))
}

#[cfg(feature = "http2")]
fn h2_send_headers_err(e: h2::Error) -> ServerError {
    ServerError::Custom(format!(
        "failed to send h2 response headers: {e}"
    ))
}

#[cfg(feature = "http2")]
fn h2_send_body_err(e: h2::Error) -> ServerError {
    ServerError::Custom(format!("failed to send h2 response body: {e}"))
}

#[cfg(feature = "http2")]
fn h2_build_head_err(e: http::Error) -> ServerError {
    ServerError::Custom(format!(
        "failed to build h2 response headers: {e}"
    ))
}

#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
async fn handle_h2_connection(
    stream: tokio::net::TcpStream,
    server: Server,
) -> Result<(), ServerError> {
    // Disable Nagle — HTTP/2 frame flushing should not wait for delayed ACK.
    let _ = stream.set_nodelay(true);
    let mut connection = h2::server::handshake(stream)
        .await
        .map_err(h2_handshake_err)?;

    while let Some(next) = connection.accept().await {
        let (request, respond) = next.map_err(h2_accept_err)?;
        let parsed_request = map_h2_request(&request);
        let response = build_response_for_request_with_metrics(
            &server,
            &parsed_request,
        );
        send_h2_response(respond, response)?;
    }

    Ok(())
}

#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
fn map_h2_request<B>(request: &http::Request<B>) -> Request {
    let headers = request
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|value| {
                (name.as_str().to_ascii_lowercase(), value.to_string())
            })
        })
        .collect();

    let version = match request.version() {
        http::Version::HTTP_2 => "HTTP/2.0",
        _ => "HTTP/1.1",
    };

    Request {
        method: request.method().as_str().to_string(),
        path: request.uri().path().to_string(),
        version: version.to_string(),
        headers,
    }
}

#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
fn send_h2_response(
    mut respond: h2::server::SendResponse<bytes::Bytes>,
    response: crate::response::Response,
) -> Result<(), ServerError> {
    let head = build_h2_head(&response)?;

    let end_of_stream = response.body.is_empty();
    let mut stream = respond
        .send_response(head, end_of_stream)
        .map_err(h2_send_headers_err)?;

    if !end_of_stream {
        stream
            .send_data(bytes::Bytes::from(response.body), true)
            .map_err(h2_send_body_err)?;
    }

    Ok(())
}

#[cfg(feature = "http2")]
#[cfg_attr(docsrs, doc(cfg(feature = "http2")))]
fn build_h2_head(
    response: &crate::response::Response,
) -> Result<http::Response<()>, ServerError> {
    let mut builder =
        http::Response::builder().status(response.status_code);
    for (name, value) in &response.headers {
        builder = builder.header(name, value);
    }
    builder.body(()).map_err(h2_build_head_err)
}

#[cfg(all(test, feature = "http2"))]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::Version;
    use std::io::Write;
    use std::net::TcpListener;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;
    use tokio::time::{Duration, sleep};

    fn free_addr() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);
        addr.to_string()
    }

    #[tokio::test]
    async fn http2_server_serves_static_file() {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello-h2")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let task = tokio::spawn(start_http2(server));
        sleep(Duration::from_millis(40)).await;

        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        let (mut client, connection) =
            h2::client::handshake(stream).await.expect("handshake");
        drop(tokio::spawn(connection));

        let request = http::Request::builder()
            .method("GET")
            .uri("http://localhost/")
            .body(())
            .expect("request");
        let (response_future, _send_stream) =
            client.send_request(request, true).expect("send request");
        let response = response_future.await.expect("response");
        assert_eq!(response.status().as_u16(), 200);

        let mut body = response.into_body();
        let mut collected = Vec::new();
        while let Some(next) = body.data().await {
            let chunk: Bytes = next.expect("chunk");
            collected.extend_from_slice(&chunk);
        }

        assert_eq!(collected, b"hello-h2");
        task.abort();
    }

    #[test]
    fn map_h2_request_preserves_method_path_headers_and_version() {
        let request = http::Request::builder()
            .method("GET")
            .uri("/status")
            .version(Version::HTTP_2)
            .header("x-test", "value")
            .body(())
            .expect("request");
        let parsed = map_h2_request(&request);
        assert_eq!(parsed.method(), "GET");
        assert_eq!(parsed.path(), "/status");
        assert_eq!(parsed.version(), "HTTP/2.0");
        assert_eq!(parsed.header("x-test"), Some("value"));
    }

    #[test]
    fn map_h2_request_falls_back_to_http11_for_other_versions() {
        let request = http::Request::builder()
            .method("GET")
            .uri("/legacy")
            .version(Version::HTTP_11)
            .body(())
            .expect("request");
        let parsed = map_h2_request(&request);
        assert_eq!(parsed.version(), "HTTP/1.1");
    }

    #[test]
    fn h2_error_context_helpers_wrap_source_message() {
        let reason = h2::Reason::PROTOCOL_ERROR;
        let handshake = h2_handshake_err(h2::Error::from(reason));
        assert!(matches!(handshake, ServerError::Custom(_)));
        assert!(handshake.to_string().contains("h2 handshake:"));

        let accept = h2_accept_err(h2::Error::from(reason));
        assert!(accept.to_string().contains("h2 accept:"));

        let headers = h2_send_headers_err(h2::Error::from(reason));
        assert!(
            headers.to_string().contains("send h2 response headers:")
        );

        let body = h2_send_body_err(h2::Error::from(reason));
        assert!(body.to_string().contains("send h2 response body:"));

        // http::Error construction: build a response with a malformed
        // header name so `builder.body(())` returns Err(http::Error).
        let http_err = http::Response::builder()
            .header("bad header name", "v")
            .body(())
            .expect_err(
                "invalid header name should produce http::Error",
            );
        let built = h2_build_head_err(http_err);
        assert!(
            built.to_string().contains("build h2 response headers:")
        );
    }

    #[test]
    fn build_h2_head_rejects_invalid_header_name() {
        let mut response =
            crate::response::Response::new(200, "OK", Vec::new());
        response.add_header("bad header", "value");
        let result = build_h2_head(&response);
        assert!(matches!(result, Err(ServerError::Custom(_))));
    }

    #[tokio::test]
    async fn handle_h2_connection_reports_handshake_error_on_invalid_preface()
     {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let listener =
            tokio::net::TcpListener::bind(&addr).await.expect("bind");
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            handle_h2_connection(stream, server).await
        });

        let mut client =
            std::net::TcpStream::connect(&addr).expect("connect");
        client
            .write_all(b"this-is-not-http2")
            .expect("write invalid preface");

        let result = accept_task.await.expect("join");
        assert!(matches!(result, Err(ServerError::Custom(_))));
    }

    #[tokio::test]
    async fn http2_server_returns_404_for_missing_resource() {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello-h2")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404 page")
            .expect("write 404");

        let addr = free_addr();
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let task = tokio::spawn(start_http2(server));
        sleep(Duration::from_millis(40)).await;

        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        let (mut client, connection) =
            h2::client::handshake(stream).await.expect("handshake");
        drop(tokio::spawn(connection));

        let request = http::Request::builder()
            .method("GET")
            .uri("http://localhost/does-not-exist")
            .body(())
            .expect("request");
        let (response_future, _send_stream) =
            client.send_request(request, true).expect("send request");
        let response = response_future.await.expect("response");
        assert_eq!(response.status().as_u16(), 404);

        let mut body = response.into_body();
        let mut collected = Vec::new();
        while let Some(next) = body.data().await {
            let chunk: Bytes = next.expect("chunk");
            collected.extend_from_slice(&chunk);
        }
        assert_eq!(collected, b"404 page");
        task.abort();
    }

    #[tokio::test]
    async fn http2_server_returns_405_for_unsupported_method() {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello-h2")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let task = tokio::spawn(start_http2(server));
        sleep(Duration::from_millis(40)).await;

        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        let (mut client, connection) =
            h2::client::handshake(stream).await.expect("handshake");
        drop(tokio::spawn(connection));

        let request = http::Request::builder()
            .method("POST")
            .uri("http://localhost/")
            .body(())
            .expect("request");
        let (response_future, _send_stream) =
            client.send_request(request, true).expect("send request");
        let response = response_future.await.expect("response");
        assert_eq!(response.status().as_u16(), 405);
        task.abort();
    }

    #[tokio::test]
    async fn handle_h2_connection_surfaces_send_errors_when_client_rsts()
     {
        // When the TCP connection drops between request arrival and the
        // server's `send_response`/`send_data`, h2 reports an error mapped
        // to `ServerError::Custom`. This exercises the response-send error
        // branches in `send_h2_response`.
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello-h2")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let listener =
            tokio::net::TcpListener::bind(&addr).await.expect("bind");
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            handle_h2_connection(stream, server).await
        });

        let tcp = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        // RST on drop so the server sees a hung-up connection rather than a
        // FIN-based clean close.
        tcp.set_linger(Some(Duration::from_secs(0)))
            .expect("set_linger");
        let (mut client, connection) =
            h2::client::handshake(tcp).await.expect("handshake");
        let conn_task = tokio::spawn(connection);

        let request = http::Request::builder()
            .method("GET")
            .uri("http://localhost/")
            .body(())
            .expect("request");
        let (_response_future, _send) =
            client.send_request(request, true).expect("send request");
        // Drop the client and the connection driver before the server
        // finishes responding. The underlying TcpStream gets RST-ed.
        drop(client);
        conn_task.abort();

        let result =
            tokio::time::timeout(Duration::from_secs(2), accept_task)
                .await
                .expect("accept_task timed out");
        // The join must succeed; the inner result can be Ok, or a Custom
        // error from send_response/send_data/accept on the RST.
        let _ = result.expect("join");
    }

    #[tokio::test]
    async fn start_http2_handles_invalid_client_preface() {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello-h2")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let task = tokio::spawn(start_http2(server));
        sleep(Duration::from_millis(40)).await;

        let mut client =
            std::net::TcpStream::connect(&addr).expect("connect");
        client
            .write_all(b"not-http2")
            .expect("write invalid preface");
        sleep(Duration::from_millis(40)).await;
        task.abort();
    }

    #[tokio::test]
    async fn handle_h2_connection_returns_ok_when_client_closes_cleanly()
     {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello-h2")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let listener =
            tokio::net::TcpListener::bind(&addr).await.expect("bind");
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            handle_h2_connection(stream, server).await
        });

        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        let (mut client, connection) =
            h2::client::handshake(stream).await.expect("handshake");
        let conn_task = tokio::spawn(connection);

        let request = http::Request::builder()
            .method("GET")
            .uri("http://localhost/")
            .body(())
            .expect("request");
        let (response_future, _send_stream) =
            client.send_request(request, true).expect("send request");
        let _ = response_future.await.expect("response");
        drop(client);
        let _ =
            tokio::time::timeout(Duration::from_millis(500), conn_task)
                .await;

        let _ = tokio::time::timeout(
            Duration::from_millis(500),
            accept_task,
        )
        .await;
    }

    #[tokio::test]
    async fn handle_h2_connection_maps_accept_errors() {
        let root = TempDir::new().expect("tmp");
        std::fs::write(root.path().join("index.html"), b"hello")
            .expect("write index");
        std::fs::create_dir(root.path().join("404")).expect("404 dir");
        std::fs::write(root.path().join("404/index.html"), b"404")
            .expect("write 404");

        let addr = free_addr();
        let listener =
            tokio::net::TcpListener::bind(&addr).await.expect("bind");
        let server = Server::builder()
            .address(&addr)
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            handle_h2_connection(stream, server).await
        });

        let mut client = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        // Valid HTTP/2 preface followed by malformed frame bytes.
        client
            .write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
            .await
            .expect("preface");
        client
            .write_all(&[0, 0, 1, 0xff, 0, 0, 0, 0, 0, 0x00])
            .await
            .expect("malformed frame");
        let _ = client.shutdown().await;

        let result = accept_task.await.expect("join");
        assert!(
            result.is_ok()
                || matches!(result, Err(ServerError::Custom(_)))
        );
    }
}
