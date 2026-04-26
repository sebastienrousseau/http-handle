// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! High-performance async-first HTTP/1 server primitives.

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use crate::error::ServerError;
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use crate::request::Request;
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use crate::response::Response;
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use crate::server::{
    ConnectionPolicy, KEEPALIVE_IDLE_TIMEOUT, MAX_KEEPALIVE_REQUESTS,
    Server, build_response_for_request_with_metrics,
};

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use std::path::{Path, PathBuf};
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use std::sync::Arc;
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use tokio::sync::Semaphore;
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
use tokio::time::{Duration, timeout};

/// Runtime limits for the high-performance server mode.
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
///
/// # Examples
///
/// ```rust
/// use http_handle::perf_server::PerfLimits;
/// let limits = PerfLimits::default();
/// assert!(limits.max_inflight > 0);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Copy, Debug)]
pub struct PerfLimits {
    /// Maximum number of concurrently processed connections.
    pub max_inflight: usize,
    /// Maximum number of queued connections waiting for a slot.
    pub max_queue: usize,
    /// Minimum file size (bytes) for sendfile fast-path attempts.
    pub sendfile_threshold_bytes: u64,
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
impl Default for PerfLimits {
    fn default() -> Self {
        Self {
            max_inflight: 256,
            max_queue: 1024,
            sendfile_threshold_bytes: 64 * 1024,
        }
    }
}

/// Starts an async-first accept loop with adaptive backpressure.
///
/// This path prioritizes throughput-per-core by avoiding a thread-per-connection model,
/// enforcing queue limits, and using a sendfile fast-path for large static files.
#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
///
/// # Examples
///
/// ```rust,no_run
/// use http_handle::perf_server::{start_high_perf, PerfLimits};
/// use http_handle::Server;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let server = Server::new("127.0.0.1:8080", ".");
/// let _ = start_high_perf(server, PerfLimits::default()).await;
/// # }
/// ```
///
/// # Errors
///
/// Returns an error when socket binding or accept fails.
///
/// # Panics
///
/// This function does not panic.
pub async fn start_high_perf(
    server: Server,
    limits: PerfLimits,
) -> Result<(), ServerError> {
    let listener = tokio::net::TcpListener::bind(server.address())
        .await
        .map_err(ServerError::from)?;

    let inflight = Arc::new(Semaphore::new(limits.max_inflight.max(1)));
    let queued = Arc::new(AtomicUsize::new(0));

    loop {
        let (stream, _addr) =
            listener.accept().await.map_err(ServerError::from)?;

        let permit = match inflight.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let queued_now =
                    queued.fetch_add(1, Ordering::SeqCst) + 1;
                if queued_now > limits.max_queue {
                    let _ = queued.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                let acquired = timeout(
                    Duration::from_millis(20),
                    inflight.clone().acquire_owned(),
                )
                .await;
                let _ = queued.fetch_sub(1, Ordering::SeqCst);
                match acquired {
                    Ok(Ok(permit)) => permit,
                    _ => continue,
                }
            }
        };

        let server_clone = server.clone();
        let limits_clone = limits;
        drop(tokio::spawn(async move {
            let _permit = permit;
            let _ = handle_async_connection(
                stream,
                &server_clone,
                &limits_clone,
            )
            .await;
        }));
    }
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
async fn handle_async_connection(
    mut stream: tokio::net::TcpStream,
    server: &Server,
    limits: &PerfLimits,
) -> Result<(), ServerError> {
    // Disable Nagle so header+body are not held by the kernel waiting
    // for a delayed ACK on small payloads.
    let _ = stream.set_nodelay(true);
    let request_timeout =
        server.request_timeout().unwrap_or(Duration::from_secs(30));

    // HTTP/1.1 persistent-connection loop. The first request gets the
    // configured per-request timeout; subsequent requests on the same
    // TCP connection get the tighter idle timeout so an inactive client
    // is reaped promptly without holding the inflight permit. Re-uses
    // ConnectionPolicy from the sync path so HTTP/1.0 and explicit
    // `Connection: close` semantics match across server entry points.
    for i in 0..MAX_KEEPALIVE_REQUESTS {
        let read_deadline = if i == 0 {
            request_timeout
        } else {
            KEEPALIVE_IDLE_TIMEOUT
        };
        let mut buffer = vec![0_u8; 16 * 1024];
        let read = match timeout(
            read_deadline,
            stream.read(&mut buffer),
        )
        .await
        {
            Ok(Ok(0)) => return Ok(()), // peer FIN
            Ok(Ok(n)) => n,
            Ok(Err(_)) | Err(_) => return Ok(()), // read error or idle timeout
        };
        buffer.truncate(read);

        let request = parse_request_from_bytes(&buffer)?;
        let policy = ConnectionPolicy::from_request(&request);

        if try_send_static_file_fast_path(
            &mut stream,
            server,
            &request,
            limits.sendfile_threshold_bytes,
            policy,
        )
        .await?
        {
            if policy == ConnectionPolicy::Close {
                return Ok(());
            }
            continue;
        }

        let mut response =
            build_response_for_request_with_metrics(server, &request);
        response.set_connection_header(policy.header_value());
        if send_response_async(&mut stream, &response).await.is_err() {
            return Ok(());
        }
        if policy == ConnectionPolicy::Close {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
fn parse_request_from_bytes(
    bytes: &[u8],
) -> Result<Request, ServerError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ServerError::invalid_request("request is not valid UTF-8")
    })?;
    let (head, _) = text.split_once("\r\n\r\n").ok_or_else(|| {
        ServerError::invalid_request("incomplete HTTP request head")
    })?;

    let mut lines = head.lines();
    let request_line = lines.next().ok_or_else(|| {
        ServerError::invalid_request("missing request line")
    })?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| ServerError::invalid_request("missing method"))?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| ServerError::invalid_request("missing path"))?
        .to_string();
    let version = parts
        .next()
        .ok_or_else(|| {
            ServerError::invalid_request("missing HTTP version")
        })?
        .to_string();

    let mut headers: Vec<(String, String)> = Vec::with_capacity(8);
    for line in lines {
        if line.is_empty() {
            break;
        }
        // SIMD ':' search via memchr; same rationale as src/request.rs.
        let bytes = line.as_bytes();
        if let Some(colon) = memchr::memchr(b':', bytes) {
            let (name, value) = line.split_at(colon);
            let value = &value[1..];
            headers.push((
                name.trim().to_ascii_lowercase(),
                value.trim().to_string(),
            ));
        }
    }

    Ok(Request {
        method,
        path,
        version,
        headers,
    })
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
async fn send_response_async(
    stream: &mut tokio::net::TcpStream,
    response: &Response,
) -> Result<(), ServerError> {
    use std::fmt::Write as _;
    // Pre-size for typical response sizes; growth is rare. Mirrors P0.A
    // on the sync path: no intermediate format!() allocations on each
    // header line — write! goes directly into the existing buffer.
    let mut header = String::with_capacity(256);
    let _ = write!(
        &mut header,
        "HTTP/1.1 {} {}\r\n",
        response.status_code, response.status_text
    );

    let mut has_content_length = false;
    let mut has_connection = false;
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
        if name.eq_ignore_ascii_case("connection") {
            has_connection = true;
        }
        let _ = write!(&mut header, "{}: {}\r\n", name, value);
    }
    if !has_content_length {
        let _ = write!(
            &mut header,
            "Content-Length: {}\r\n",
            response.body.len()
        );
    }
    if !has_connection {
        header.push_str("Connection: close\r\n");
    }
    header.push_str("\r\n");

    stream
        .write_all(header.as_bytes())
        .await
        .map_err(ServerError::from)?;
    if !response.body.is_empty() {
        stream
            .write_all(&response.body)
            .await
            .map_err(ServerError::from)?;
    }
    stream.flush().await.map_err(ServerError::from)?;
    Ok(())
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
async fn try_send_static_file_fast_path(
    stream: &mut tokio::net::TcpStream,
    server: &Server,
    request: &Request,
    sendfile_threshold_bytes: u64,
    policy: ConnectionPolicy,
) -> Result<bool, ServerError> {
    if request.method() != "GET" && request.method() != "HEAD" {
        return Ok(false);
    }
    if request.header("range").is_some() {
        return Ok(false);
    }

    let Some(file_path) =
        resolve_static_path(server.document_root(), request.path())
            .await
    else {
        return Ok(false);
    };

    let (serving_path, encoding) =
        negotiate_precompressed(&file_path, request);
    let metadata = tokio::fs::metadata(&serving_path)
        .await
        .map_err(ServerError::from)?;
    let len = metadata.len();

    let mut headers = Vec::new();
    headers.push(("Content-Type", content_type_for_path(&file_path)));
    headers.push(("Accept-Ranges", "bytes"));
    if let Some(enc) = encoding {
        headers.push(("Content-Encoding", enc));
        headers.push(("Vary", "Accept-Encoding"));
    }
    if is_probably_immutable_asset(request.path()) {
        headers.push((
            "Cache-Control",
            "public, max-age=31536000, immutable",
        ));
    }

    let mut head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: {}\r\n",
        policy.header_value()
    );
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    stream
        .write_all(head.as_bytes())
        .await
        .map_err(ServerError::from)?;

    if request.method() == "HEAD" {
        stream.flush().await.map_err(ServerError::from)?;
        return Ok(true);
    }

    if len >= sendfile_threshold_bytes {
        #[cfg(unix)]
        {
            if try_sendfile_unix(stream, &serving_path, len).await? {
                stream.flush().await.map_err(ServerError::from)?;
                return Ok(true);
            }
        }
    }

    let mut file = tokio::fs::File::open(&serving_path)
        .await
        .map_err(ServerError::from)?;
    let _bytes_copied = tokio::io::copy(&mut file, stream)
        .await
        .map_err(ServerError::from)?;
    stream.flush().await.map_err(ServerError::from)?;
    Ok(true)
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
async fn resolve_static_path(
    root: &Path,
    request_path: &str,
) -> Option<PathBuf> {
    let canonical_root = tokio::fs::canonicalize(root).await.ok()?;
    let mut path = root.to_path_buf();
    let rel = request_path.trim_start_matches('/');

    if rel.is_empty() {
        path.push("index.html");
    } else {
        for part in rel.split('/') {
            if part == ".." {
                let _ = path.pop();
            } else {
                path.push(part);
            }
        }
    }

    let resolved = tokio::fs::canonicalize(&path).await.ok()?;
    if !resolved.starts_with(canonical_root) {
        return None;
    }

    let meta = tokio::fs::metadata(&resolved).await.ok()?;
    if meta.is_dir() {
        let index = resolved.join("index.html");
        let index_meta = tokio::fs::metadata(&index).await.ok()?;
        if index_meta.is_file() {
            return Some(index);
        }
        return None;
    }

    if meta.is_file() { Some(resolved) } else { None }
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
fn negotiate_precompressed(
    path: &Path,
    request: &Request,
) -> (PathBuf, Option<&'static str>) {
    let mut serving_path = path.to_path_buf();
    let mut encoding = None;

    if let Some(accept) = request.header("accept-encoding") {
        if accept.contains("br") {
            let candidate =
                PathBuf::from(format!("{}.br", path.display()));
            if candidate.is_file() {
                serving_path = candidate;
                encoding = Some("br");
                return (serving_path, encoding);
            }
        }
        if accept.contains("zstd") || accept.contains("zst") {
            let candidate =
                PathBuf::from(format!("{}.zst", path.display()));
            if candidate.is_file() {
                serving_path = candidate;
                encoding = Some("zstd");
                return (serving_path, encoding);
            }
        }
        if accept.contains("gzip") {
            let candidate =
                PathBuf::from(format!("{}.gz", path.display()));
            if candidate.is_file() {
                serving_path = candidate;
                encoding = Some("gzip");
            }
        }
    }

    (serving_path, encoding)
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
    {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

#[cfg(feature = "high-perf")]
#[cfg_attr(docsrs, doc(cfg(feature = "high-perf")))]
fn is_probably_immutable_asset(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    let Some((stem, _ext)) = file.rsplit_once('.') else {
        return false;
    };
    let Some(hash) = stem.rsplit('-').next() else {
        return false;
    };
    hash.len() >= 8 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(all(
    feature = "high-perf",
    any(target_os = "linux", target_os = "android")
))]
async fn try_sendfile_unix(
    stream: &tokio::net::TcpStream,
    path: &Path,
    len: u64,
) -> Result<bool, ServerError> {
    use std::os::fd::AsRawFd;
    // File::open is a blocking syscall; run it on the blocking pool so
    // the Tokio reactor thread is never stalled opening files.
    let path_owned = path.to_path_buf();
    let file = tokio::task::spawn_blocking(move || {
        std::fs::File::open(path_owned)
    })
    .await
    .map_err(|e| ServerError::TaskFailed(e.to_string()))?
    .map_err(ServerError::from)?;
    let mut offset: libc::off_t = 0;
    let mut sent: u64 = 0;

    while sent < len {
        let remaining = (len - sent) as usize;
        let chunk = remaining.min(1 << 20);
        // Safety: both fds are owned for the duration of this call —
        // `stream` is borrowed from the caller (the TcpStream lives on
        // the stack frame above) and `file` is the local std::fs::File
        // we just opened. `offset` is a local `libc::off_t` we write
        // through. `chunk` is bounded above by `len - sent` and below
        // by 1 (the loop guard `sent < len`). The kernel either fills
        // the requested transfer or returns the count actually sent;
        // we handle the negative-rc and EAGAIN cases below.
        #[allow(unsafe_code)]
        let rc = unsafe {
            libc::sendfile(
                stream.as_raw_fd(),
                file.as_raw_fd(),
                &mut offset,
                chunk,
            )
        };
        if rc == 0 {
            break;
        }
        if rc < 0 {
            let err = std::io::Error::last_os_error();
            if matches!(err.raw_os_error(), Some(libc::EAGAIN)) {
                tokio::task::yield_now().await;
                continue;
            }
            return Ok(false);
        }
        sent = sent.saturating_add(rc as u64);
    }

    Ok(sent > 0)
}

#[cfg(all(
    feature = "high-perf",
    unix,
    not(any(target_os = "linux", target_os = "android"))
))]
async fn try_sendfile_unix(
    _stream: &tokio::net::TcpStream,
    _path: &Path,
    _len: u64,
) -> Result<bool, ServerError> {
    Ok(false)
}

#[cfg(all(test, feature = "high-perf"))]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::time::Duration;

    #[test]
    fn immutable_asset_detection() {
        assert!(is_probably_immutable_asset("/assets/app-abcdef12.js"));
        assert!(!is_probably_immutable_asset("/assets/app.js"));
    }

    #[test]
    fn parse_request_from_bytes_parses_headers() {
        let request = parse_request_from_bytes(
            b"GET / HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: gzip\r\n\r\n",
        )
        .expect("parse");
        assert_eq!(request.method(), "GET");
        assert_eq!(request.path(), "/");
        assert_eq!(request.header("host"), Some("localhost"));
        assert_eq!(request.header("accept-encoding"), Some("gzip"));
    }

    #[test]
    fn parse_request_from_bytes_rejects_invalid_inputs() {
        assert!(parse_request_from_bytes(b"\xFF").is_err());
        assert!(
            parse_request_from_bytes(b"GET / HTTP/1.1\r\n").is_err()
        );
        assert!(
            parse_request_from_bytes(b"/ HTTP/1.1\r\n\r\n").is_err()
        );
        assert!(parse_request_from_bytes(b"\r\n\r\n").is_err());
        assert!(parse_request_from_bytes(b"GET\r\n\r\n").is_err());
        assert!(parse_request_from_bytes(b"GET / \r\n\r\n").is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resolve_static_path_and_content_type_behave() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("index.html"), "ok").expect("write");
        std::fs::create_dir(root.join("nested")).expect("mkdir");
        std::fs::write(root.join("nested").join("index.html"), "n")
            .expect("write");

        let p1 =
            resolve_static_path(root, "/").await.expect("root index");
        assert!(p1.ends_with("index.html"));
        let p2 = resolve_static_path(root, "/nested")
            .await
            .expect("nested index");
        assert!(p2.ends_with("nested/index.html"));
        assert!(
            resolve_static_path(root, "/../../etc/passwd")
                .await
                .is_none()
        );

        assert_eq!(
            content_type_for_path(Path::new("a.html")),
            "text/html"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.css")),
            "text/css"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.js")),
            "application/javascript"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.bin")),
            "application/octet-stream"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.json")),
            "application/json"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.wasm")),
            "application/wasm"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.svg")),
            "image/svg+xml"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.png")),
            "image/png"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.jpg")),
            "image/jpeg"
        );
        assert_eq!(
            content_type_for_path(Path::new("a.gif")),
            "image/gif"
        );
    }

    #[test]
    fn negotiate_precompressed_prefers_br_then_zstd_then_gzip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("index.html");
        std::fs::write(&base, "x").expect("base");

        let headers =
            vec![("accept-encoding".to_string(), "gzip".to_string())];
        let req_gz = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };
        std::fs::write(format!("{}.gz", base.display()), "x")
            .expect("gz");
        let (p, e) = negotiate_precompressed(&base, &req_gz);
        assert!(p.ends_with("index.html.gz"));
        assert_eq!(e, Some("gzip"));

        std::fs::write(format!("{}.zst", base.display()), "x")
            .expect("zst");
        let headers = vec![(
            "accept-encoding".to_string(),
            "zstd,gzip".to_string(),
        )];
        let req_zst = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };
        let (p, e) = negotiate_precompressed(&base, &req_zst);
        assert!(p.ends_with("index.html.zst"));
        assert_eq!(e, Some("zstd"));

        std::fs::write(format!("{}.br", base.display()), "x")
            .expect("br");
        let headers = vec![(
            "accept-encoding".to_string(),
            "br,zstd,gzip".to_string(),
        )];
        let req_br = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };
        let (p, e) = negotiate_precompressed(&base, &req_br);
        assert!(p.ends_with("index.html.br"));
        assert_eq!(e, Some("br"));

        let headers =
            vec![("accept-encoding".to_string(), "gzip".to_string())];
        let req_gz_missing = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };
        std::fs::remove_file(format!("{}.gz", base.display()))
            .expect("remove gz");
        let (p, e) = negotiate_precompressed(&base, &req_gz_missing);
        assert!(p.ends_with("index.html"));
        assert_eq!(e, None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_send_static_file_fast_path_serves_get_and_head() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("app-abcdef12.js"),
            "console.log('ok');",
        )
        .expect("write");

        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.to_string_lossy().as_ref())
            .build()
            .expect("server");
        let request = Request {
            method: "GET".into(),
            path: "/app-abcdef12.js".into(),
            version: "HTTP/1.1".into(),
            headers: Vec::new(),
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");

        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let mut stream = server_stream;
            try_send_static_file_fast_path(
                &mut stream,
                &server_clone,
                &request,
                u64::MAX,
                ConnectionPolicy::Close,
            )
            .await
            .expect("send")
        });

        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        assert!(server_task.await.expect("join"));

        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("HTTP/1.1 200 OK"));
        assert!(text.contains(
            "Cache-Control: public, max-age=31536000, immutable"
        ));
        assert!(text.contains("application/javascript"));

        let request_head = Request {
            method: "HEAD".into(),
            path: "/app-abcdef12.js".into(),
            version: "HTTP/1.1".into(),
            headers: Vec::new(),
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");
        let server_clone = server.clone();
        let server_task = tokio::spawn(async move {
            let mut stream = server_stream;
            try_send_static_file_fast_path(
                &mut stream,
                &server_clone,
                &request_head,
                u64::MAX,
                ConnectionPolicy::Close,
            )
            .await
            .expect("send")
        });
        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        assert!(server_task.await.expect("join"));
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("HTTP/1.1 200 OK"));
        assert!(!text.contains("console.log('ok')"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_send_static_file_fast_path_rejects_non_get_and_range()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("index.html"), "ok").expect("write");

        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.to_string_lossy().as_ref())
            .build()
            .expect("server");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (mut server_stream, _) =
            listener.accept().await.expect("accept");
        let _client = client_task.await.expect("join");

        let post_req = Request {
            method: "POST".into(),
            path: "/index.html".into(),
            version: "HTTP/1.1".into(),
            headers: Vec::new(),
        };
        assert!(
            !try_send_static_file_fast_path(
                &mut server_stream,
                &server,
                &post_req,
                u64::MAX,
                ConnectionPolicy::Close,
            )
            .await
            .expect("ok")
        );

        let headers = vec![("range".into(), "bytes=0-3".into())];
        let range_req = Request {
            method: "GET".into(),
            path: "/index.html".into(),
            version: "HTTP/1.1".into(),
            headers,
        };
        assert!(
            !try_send_static_file_fast_path(
                &mut server_stream,
                &server,
                &range_req,
                u64::MAX,
                ConnectionPolicy::Close,
            )
            .await
            .expect("ok")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn send_response_async_adds_default_headers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (mut server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");

        let response = Response::new(200, "OK", b"hello".to_vec());
        send_response_async(&mut server_stream, &response)
            .await
            .expect("send");
        drop(server_stream);

        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("HTTP/1.1 200 OK"));
        assert!(text.contains("Content-Length: 5"));
        assert!(text.contains("Connection: close"));
        assert!(text.ends_with("hello"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn send_response_async_keeps_existing_headers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (mut server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");

        let mut response = Response::new(204, "No Content", Vec::new());
        response.headers.push(("Content-Length".into(), "0".into()));
        response
            .headers
            .push(("Connection".into(), "keep-alive".into()));
        send_response_async(&mut server_stream, &response)
            .await
            .expect("send");
        drop(server_stream);

        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("Content-Length: 0"));
        assert!(text.contains("Connection: keep-alive"));
        assert!(!text.contains("Connection: close"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn handle_async_connection_rejects_invalid_utf8() {
        let dir = tempfile::tempdir().expect("tempdir");
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(dir.path().to_string_lossy().as_ref())
            .build()
            .expect("server");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(addr)
                .await
                .expect("connect");
            stream.write_all(b"\xFF\xFE").await.expect("write");
            stream
        });
        let (server_stream, _) =
            listener.accept().await.expect("accept");
        let _client = client_task.await.expect("join");

        let err = handle_async_connection(
            server_stream,
            &server,
            &PerfLimits::default(),
        )
        .await
        .expect_err("invalid utf8 should fail");
        assert!(err.to_string().contains("Invalid request"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn handle_async_connection_returns_ok_on_clean_close() {
        let dir = tempfile::tempdir().expect("tempdir");
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(dir.path().to_string_lossy().as_ref())
            .build()
            .expect("server");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr)
                .await
                .expect("connect");
            drop(stream);
        });
        let (server_stream, _) =
            listener.accept().await.expect("accept");
        client_task.await.expect("join");

        handle_async_connection(
            server_stream,
            &server,
            &PerfLimits::default(),
        )
        .await
        .expect("clean close");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn handle_async_connection_sends_built_response() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir(root.join("404")).expect("404 dir");
        std::fs::write(root.join("404/index.html"), "not found")
            .expect("404");
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.to_string_lossy().as_ref())
            .build()
            .expect("server");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(addr)
                .await
                .expect("connect");
            stream
                .write_all(
                    b"GET /missing.txt HTTP/1.1\r\nHost: localhost\r\n\r\n",
                )
                .await
                .expect("write");
            stream
        });
        let (server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");
        handle_async_connection(
            server_stream,
            &server,
            &PerfLimits::default(),
        )
        .await
        .expect("handled");

        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("HTTP/1.1"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fast_path_includes_precompressed_encoding_headers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("index.html"), "plain").expect("base");
        std::fs::write(root.join("index.html.gz"), "gzdata")
            .expect("gz");
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.to_string_lossy().as_ref())
            .build()
            .expect("server");

        let headers =
            vec![("accept-encoding".to_string(), "gzip".to_string())];
        let req = Request {
            method: "GET".into(),
            path: "/index.html".into(),
            version: "HTTP/1.1".into(),
            headers,
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (mut server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");

        assert!(
            try_send_static_file_fast_path(
                &mut server_stream,
                &server,
                &req,
                u64::MAX,
                ConnectionPolicy::Close,
            )
            .await
            .expect("served")
        );
        drop(server_stream);
        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("Content-Encoding: gzip"));
        assert!(text.contains("Vary: Accept-Encoding"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resolve_static_path_handles_missing_dir_index_and_immutable_edge_cases()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir(root.join("dir-no-index")).expect("mkdir");
        assert!(
            resolve_static_path(root, "/dir-no-index").await.is_none()
        );
        assert!(!is_probably_immutable_asset("/assets/noext"));
        assert!(!is_probably_immutable_asset("/assets/file.js"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_send_static_file_fast_path_missing_file_returns_false()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(dir.path().to_string_lossy().as_ref())
            .build()
            .expect("server");
        let request = Request {
            method: "GET".into(),
            path: "/missing.txt".into(),
            version: "HTTP/1.1".into(),
            headers: Vec::new(),
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (mut server_stream, _) =
            listener.accept().await.expect("accept");
        let _client = client_task.await.expect("join");

        let served = try_send_static_file_fast_path(
            &mut server_stream,
            &server,
            &request,
            u64::MAX,
            ConnectionPolicy::Close,
        )
        .await
        .expect("missing file should map to false");
        assert!(!served);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[tokio::test(flavor = "current_thread")]
    async fn try_sendfile_unix_sends_file_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("blob.bin");
        let payload = b"abcdef123456";
        std::fs::write(&path, payload).expect("write");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");

        let sent = try_sendfile_unix(
            &server_stream,
            &path,
            payload.len() as u64,
        )
        .await
        .expect("sendfile");
        assert!(sent);
        drop(server_stream);

        let mut got = Vec::new();
        let _ = client.read_to_end(&mut got).await.expect("read");
        assert_eq!(got, payload);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn start_high_perf_accepts_and_serves_then_can_abort() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("index.html"), "ok")
            .expect("write");

        let probe = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("probe bind");
        let addr = probe.local_addr().expect("probe addr");
        drop(probe);

        let server = Server::builder()
            .address(&addr.to_string())
            .document_root(dir.path().to_string_lossy().as_ref())
            .build()
            .expect("server");
        let limits = PerfLimits {
            max_inflight: 1,
            max_queue: 1,
            sendfile_threshold_bytes: u64::MAX,
        };

        let task = tokio::spawn(async move {
            let _ = start_high_perf(server, limits).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut client = tokio::net::TcpStream::connect(addr)
            .await
            .expect("connect");
        client
            .write_all(
                b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .await
            .expect("write");
        let mut buf = vec![0_u8; 512];
        let read =
            timeout(Duration::from_secs(1), client.read(&mut buf))
                .await
                .expect("timed read")
                .expect("read");
        assert!(read > 0);
        let text = String::from_utf8_lossy(&buf[..read]);
        assert!(text.contains("HTTP/1.1 200 OK"));

        task.abort();
        let join = task.await;
        assert!(join.is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn start_high_perf_drops_connections_when_queue_is_full() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("index.html"), "ok")
            .expect("write");

        let probe = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("probe bind");
        let addr = probe.local_addr().expect("probe addr");
        drop(probe);

        let server = Server::builder()
            .address(&addr.to_string())
            .document_root(dir.path().to_string_lossy().as_ref())
            .build()
            .expect("server");
        // One in-flight, zero queued: second concurrent connect must be
        // rejected via the `queued_now > max_queue` branch.
        let limits = PerfLimits {
            max_inflight: 1,
            max_queue: 0,
            sendfile_threshold_bytes: u64::MAX,
        };

        let task = tokio::spawn(async move {
            let _ = start_high_perf(server, limits).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Hold the single in-flight slot by connecting but never sending a
        // request — the async handler stays blocked in `read` until timeout.
        let _hold = tokio::net::TcpStream::connect(addr)
            .await
            .expect("first connect");
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Fire multiple short-lived connections; each should be accepted
        // then immediately dropped by the server (queue full / acquire timeout).
        let mut dropped = 0_usize;
        for _ in 0..8 {
            let mut probe_stream = tokio::net::TcpStream::connect(addr)
                .await
                .expect("probe connect");
            // The server drops the accepted socket in its `continue`, so the
            // read end returns EOF quickly.
            let mut buf = [0_u8; 8];
            let read = timeout(
                Duration::from_millis(200),
                probe_stream.read(&mut buf),
            )
            .await;
            if matches!(read, Ok(Ok(0))) {
                dropped += 1;
            }
        }
        assert!(
            dropped > 0,
            "expected at least one connection to be dropped by queue guard",
        );

        task.abort();
        let _ = task.await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn start_high_perf_falls_through_queue_timeout_path() {
        // Exercise the `queued_now <= max_queue` branch where the connection
        // waits on `acquire_owned` with a bounded timeout. A single in-flight
        // slot is held indefinitely so queued connects never acquire; they
        // drop after the 20ms acquire timeout.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("index.html"), "ok")
            .expect("write");
        let probe = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("probe bind");
        let addr = probe.local_addr().expect("probe addr");
        drop(probe);

        let server = Server::builder()
            .address(&addr.to_string())
            .document_root(dir.path().to_string_lossy().as_ref())
            .build()
            .expect("server");
        let limits = PerfLimits {
            max_inflight: 1,
            // Allow one queued connect so `queued_now <= max_queue` and we hit
            // the timeout-acquire branch.
            max_queue: 4,
            sendfile_threshold_bytes: u64::MAX,
        };

        let task = tokio::spawn(async move {
            let _ = start_high_perf(server, limits).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Hold the in-flight slot.
        let _hold = tokio::net::TcpStream::connect(addr)
            .await
            .expect("first connect");
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Queue up a few more — each waits on the 20ms acquire timeout
        // then gets dropped.
        for _ in 0..3 {
            let mut probe_stream = tokio::net::TcpStream::connect(addr)
                .await
                .expect("probe connect");
            let mut buf = [0_u8; 8];
            let _ = timeout(
                Duration::from_millis(200),
                probe_stream.read(&mut buf),
            )
            .await;
        }

        task.abort();
        let _ = task.await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_send_static_file_fast_path_invokes_sendfile_threshold()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let body: Vec<u8> = (0..2048_u32).map(|i| i as u8).collect();
        std::fs::write(root.join("blob.bin"), &body).expect("write");

        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.to_string_lossy().as_ref())
            .build()
            .expect("server");
        let request = Request {
            method: "GET".into(),
            path: "/blob.bin".into(),
            version: "HTTP/1.1".into(),
            headers: Vec::new(),
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client_task = tokio::spawn(async move {
            tokio::net::TcpStream::connect(addr).await.expect("connect")
        });
        let (mut server_stream, _) =
            listener.accept().await.expect("accept");
        let mut client = client_task.await.expect("join");

        // Threshold = 0 forces the sendfile fast-path branch. On Linux it
        // succeeds; on other Unix platforms it falls through to tokio::io::copy.
        let served = try_send_static_file_fast_path(
            &mut server_stream,
            &server,
            &request,
            0,
            ConnectionPolicy::Close,
        )
        .await
        .expect("served");
        assert!(served);
        drop(server_stream);

        let mut bytes = Vec::new();
        let _ = client.read_to_end(&mut bytes).await.expect("read");
        let head_end = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("header terminator");
        let head_text =
            String::from_utf8_lossy(&bytes[..head_end]).to_string();
        assert!(head_text.contains("HTTP/1.1 200 OK"));
        assert_eq!(&bytes[head_end + 4..], body.as_slice());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn try_sendfile_unix_non_linux_returns_false() {
        // The non-Linux/Android Unix fallback unconditionally returns `Ok(false)`.
        // Linux has its own impl so we skip the assertion there.
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("f.bin");
            std::fs::write(&path, b"x").expect("write");
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            let addr = listener.local_addr().expect("addr");
            drop(tokio::spawn(async move {
                tokio::net::TcpStream::connect(addr).await.expect("c")
            }));
            let (server_stream, _) =
                listener.accept().await.expect("accept");
            let sent = try_sendfile_unix(&server_stream, &path, 1)
                .await
                .expect("stub");
            assert!(!sent);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resolve_static_path_rejects_symlink_escape() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("root");
        std::fs::create_dir(&root).expect("mkroot");
        let outside = dir.path().join("outside");
        std::fs::create_dir(&outside).expect("mkoutside");
        std::fs::write(outside.join("secret.txt"), "shh")
            .expect("write secret");
        #[cfg(unix)]
        {
            let link = root.join("link.txt");
            std::os::unix::fs::symlink(
                outside.join("secret.txt"),
                &link,
            )
            .expect("symlink");
            assert!(
                resolve_static_path(&root, "/link.txt").await.is_none(),
                "symlink pointing outside root must not resolve",
            );
        }
        #[cfg(not(unix))]
        let _ = outside;
    }
}
