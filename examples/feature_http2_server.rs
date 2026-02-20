//! HTTP/2 (h2c) server example that serves one request.

#[cfg(feature = "http2")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::Server;
    use http_handle::http2_server::start_http2;
    use std::net::TcpListener;
    use tokio::time::{Duration, sleep};

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);

    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("index.html"), b"hello-http2")?;
    std::fs::create_dir(root.path().join("404"))?;
    std::fs::write(root.path().join("404/index.html"), b"404")?;

    let server = Server::builder()
        .address(&addr.to_string())
        .document_root(root.path().to_str().ok_or("invalid path")?)
        .build()?;

    let task = tokio::spawn(start_http2(server));
    sleep(Duration::from_millis(40)).await;

    let stream = tokio::net::TcpStream::connect(addr).await?;
    let (mut client, connection) =
        h2::client::handshake(stream).await?;
    drop(tokio::spawn(async move {
        let _ = connection.await;
    }));

    let request = http::Request::builder()
        .method("GET")
        .uri("http://localhost/")
        .body(())?;
    let (response_future, _) = client.send_request(request, true)?;
    let response = response_future.await?;
    println!("HTTP/2 status: {}", response.status());

    let mut body = response.into_body();
    while let Some(next) = body.data().await {
        let chunk = next?;
        println!("chunk: {}", String::from_utf8_lossy(&chunk));
    }

    task.abort();
    Ok(())
}

#[cfg(not(feature = "http2"))]
fn main() {
    eprintln!(
        "Enable the 'http2' feature: cargo run --example feature_http2_server --features http2"
    );
}
