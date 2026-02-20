//! HTTP/2 server entrypoints (feature-gated).
//!
//! This module provides a clear-text HTTP/2 (h2c) accept loop that reuses
//! the existing request/response logic from the main server module.

#[cfg(feature = "http2")]
use crate::error::ServerError;
#[cfg(feature = "http2")]
use crate::request::Request;
#[cfg(feature = "http2")]
use crate::server::{Server, build_response_for_request_with_metrics};

/// Starts an HTTP/2 (h2c) accept loop backed by Tokio.
///
/// Each accepted TCP connection is upgraded to an `h2` server connection and
/// each stream is handled using the same request->response logic used by
/// the HTTP/1 server.
#[cfg(feature = "http2")]
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
async fn handle_h2_connection(
    stream: tokio::net::TcpStream,
    server: Server,
) -> Result<(), ServerError> {
    let mut connection =
        h2::server::handshake(stream).await.map_err(|e| {
            ServerError::Custom(format!("h2 handshake: {e}"))
        })?;

    while let Some(next) = connection.accept().await {
        let (request, respond) = next.map_err(|e| {
            ServerError::Custom(format!("h2 accept: {e}"))
        })?;
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
fn map_h2_request(request: &http::Request<h2::RecvStream>) -> Request {
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
fn send_h2_response(
    mut respond: h2::server::SendResponse<bytes::Bytes>,
    response: crate::response::Response,
) -> Result<(), ServerError> {
    let mut builder =
        http::Response::builder().status(response.status_code);
    for (name, value) in &response.headers {
        builder = builder.header(name, value);
    }
    let head = builder.body(()).map_err(|error| {
        ServerError::Custom(format!(
            "failed to build h2 response headers: {error}"
        ))
    })?;

    let end_of_stream = response.body.is_empty();
    let mut stream = respond
        .send_response(head, end_of_stream)
        .map_err(|error| {
            ServerError::Custom(format!(
                "failed to send h2 response headers: {error}"
            ))
        })?;

    if !end_of_stream {
        stream
            .send_data(bytes::Bytes::from(response.body), true)
            .map_err(|error| {
                ServerError::Custom(format!(
                    "failed to send h2 response body: {error}"
                ))
            })?;
    }

    Ok(())
}

#[cfg(all(test, feature = "http2"))]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::net::TcpListener;
    use tempfile::TempDir;
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
        drop(tokio::spawn(async move {
            let _ = connection.await;
        }));

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
}
