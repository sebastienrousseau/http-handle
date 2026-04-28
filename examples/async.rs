// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `async` feature: hardened `run_blocking` + `start_async` server.
//!
//! Run: `cargo run --example async --features async`

#[cfg(feature = "async")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "async")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    use http_handle::Server;
    use http_handle::async_runtime::run_blocking;
    use http_handle::async_server::start_async;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use tokio::time::{Duration, sleep};

    support::header("http-handle -- async");

    let blocking_answer =
        run_blocking(|| Ok::<_, http_handle::ServerError>(6 * 7))
            .await
            .expect("run_blocking");
    support::task_with_output(
        "run_blocking moves CPU work off the reactor thread",
        || vec![format!("answer = {blocking_answer}")],
    );

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("index.html"), b"hello-async")
        .expect("seed");
    std::fs::create_dir(root.path().join("404")).expect("404 dir");
    std::fs::write(root.path().join("404/index.html"), b"404")
        .expect("404 page");

    let server = Server::builder()
        .address(&addr.to_string())
        .document_root(root.path().to_str().expect("path"))
        .build()
        .expect("build");
    let task = tokio::spawn(start_async(server));
    sleep(Duration::from_millis(40)).await;

    support::task_with_output(
        "start_async serves one GET via tokio::net",
        || {
            let mut stream = TcpStream::connect(addr).expect("connect");
            stream
                .write_all(
                    b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                )
                .expect("write");
            let mut buffer = [0_u8; 512];
            let read = stream.read(&mut buffer).unwrap_or(0);
            String::from_utf8_lossy(&buffer[..read])
                .lines()
                .take(4)
                .map(str::to_string)
                .collect()
        },
    );

    task.abort();
    support::summary(2);
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!(
        "Enable the 'async' feature: cargo run --example async --features async"
    );
}
