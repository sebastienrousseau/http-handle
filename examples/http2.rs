// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `http2` feature: cleartext h2c server serving one request.
//!
//! Run: `cargo run --example http2 --features http2`

#[cfg(feature = "http2")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "http2")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    use http_handle::Server;
    use http_handle::http2_server::start_http2;
    use std::net::TcpListener;
    use tokio::time::{Duration, sleep};

    support::header("http-handle -- http2");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);

    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("index.html"), b"hello-http2")
        .expect("seed");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("404 page");

    let server = Server::builder()
        .address(&addr.to_string())
        .document_root(root.path().to_str().expect("path"))
        .build()
        .expect("build");

    let task = tokio::spawn(start_http2(server));
    sleep(Duration::from_millis(40)).await;

    let stream =
        tokio::net::TcpStream::connect(addr).await.expect("connect");
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
    let (response_future, _) =
        client.send_request(request, true).expect("send");
    let response = response_future.await.expect("await response");
    let status = response.status();

    let mut body = response.into_body();
    let mut bytes = Vec::new();
    while let Some(chunk) = body.data().await {
        if let Ok(c) = chunk {
            bytes.extend_from_slice(&c);
        }
    }
    let rendered = String::from_utf8_lossy(&bytes).into_owned();

    support::task_with_output(
        "h2c handshake + GET / returns the framed body",
        || {
            vec![
                format!("status = {status}"),
                format!("body   = {rendered:?}"),
            ]
        },
    );

    task.abort();
    support::summary(1);
}

#[cfg(not(feature = "http2"))]
fn main() {
    eprintln!(
        "Enable the 'http2' feature: cargo run --example http2 --features http2"
    );
}
