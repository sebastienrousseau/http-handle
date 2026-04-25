// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Async server example that serves one request.

#[cfg(feature = "async")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::Server;
    use http_handle::async_server::start_async;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use tokio::time::{Duration, sleep};

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);

    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("index.html"), b"hello-async")?;
    std::fs::create_dir(root.path().join("404"))?;
    std::fs::write(root.path().join("404/index.html"), b"404")?;

    let server = Server::builder()
        .address(&addr.to_string())
        .document_root(root.path().to_str().ok_or("invalid path")?)
        .build()?;

    let task = tokio::spawn(start_async(server));
    sleep(Duration::from_millis(40)).await;

    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")?;

    let mut buf = [0_u8; 512];
    let read = stream.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf[..read]));

    task.abort();
    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!(
        "Enable the 'async' feature: cargo run --example feature_async_server --features async"
    );
}
