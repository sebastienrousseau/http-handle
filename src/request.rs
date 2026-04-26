// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

// src/request.rs

//! HTTP/1.x request parsing and validation.
//!
//! Use this module to convert raw stream input into typed request data with bounded parsing,
//! header normalization, and explicit malformed-request errors.

use crate::error::ServerError;
use std::fmt;
use std::io::{self, BufRead, BufReader};
use std::net::TcpStream;
use std::time::Duration;

/// Maximum length allowed for the request line (8KB).
/// This includes the method, path, version, and the two spaces between them, but not the trailing \r\n.
const MAX_REQUEST_LINE_LENGTH: usize = 8190;

/// Number of parts expected in a valid HTTP request line.
const REQUEST_PARTS: usize = 3;

/// Timeout duration for reading from the TCP stream (in seconds).
const TIMEOUT_SECONDS: u64 = 30;
/// Maximum number of accepted request headers.
const MAX_HEADER_COUNT: usize = 100;
/// Maximum allowed length for a single header line.
const MAX_HEADER_LINE_LENGTH: usize = 8192;
/// Maximum cumulative bytes for all headers.
const MAX_HEADER_BYTES: usize = 64 * 1024;

fn map_timeout_error(error: io::Error) -> ServerError {
    ServerError::invalid_request(format!(
        "Failed to set read timeout: {}",
        error
    ))
}

fn map_read_error(error: io::Error) -> ServerError {
    ServerError::invalid_request(format!(
        "Failed to read request line: {}",
        error
    ))
}

/// Represents a parsed HTTP/1.x request line and headers.
///
/// You receive this type after successful stream parsing. It is the primary request model
/// used by the synchronous server path and shared response-generation helpers.
///
/// # Examples
///
/// ```rust
/// use http_handle::request::Request;
///
/// let request = Request {
///     method: "GET".to_string(),
///     path: "/".to_string(),
///     version: "HTTP/1.1".to_string(),
///     headers: Vec::new(),
/// };
/// assert_eq!(request.method(), "GET");
/// ```
///
/// # Panics
///
/// This type does not panic on construction.
#[doc(alias = "http request")]
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// HTTP method of the request.
    pub method: String,
    /// Requested path.
    pub path: String,
    /// HTTP version of the request.
    pub version: String,
    /// Parsed request headers (header-name lowercased).
    ///
    /// Stored as `Vec<(String, String)>` rather than a `HashMap` —
    /// realistic request payloads carry well under 32 headers, so a
    /// linear scan in `Request::header` outperforms hashing for both
    /// lookup latency and per-request allocator pressure (no hash table
    /// to grow + rehash).
    pub headers: Vec<(String, String)>,
}

impl Request {
    /// Parses a request line and headers from a `TcpStream`.
    ///
    /// This method reads the first line of an HTTP request from the given TCP stream,
    /// parses it, and constructs a `Request` instance if the input is valid.
    ///
    /// # Arguments
    ///
    /// * `stream` - A reference to the `TcpStream` from which the request will be read.
    ///
    /// # Returns
    ///
    /// * `Ok(Request)` - If the request is valid and successfully parsed.
    /// * `Err(ServerError)` - If the request is malformed, cannot be read, or is invalid.
    ///
    /// # Errors
    ///
    /// This function returns a `ServerError::InvalidRequest` error if:
    /// - The request line is too long (exceeds `MAX_REQUEST_LINE_LENGTH`)
    /// - The request line does not contain exactly three parts
    /// - The HTTP method is not recognized
    /// - The request path does not start with a forward slash (except `OPTIONS *`)
    /// - The HTTP version is not supported (only HTTP/1.0 and HTTP/1.1 are accepted)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::net::TcpStream;
    /// use http_handle::request::Request;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").expect("connect");
    /// let parsed = Request::from_stream(&stream);
    /// assert!(parsed.is_ok() || parsed.is_err());
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not intentionally panic.
    #[doc(alias = "parse")]
    #[doc(alias = "from tcp")]
    pub fn from_stream(
        stream: &TcpStream,
    ) -> Result<Self, ServerError> {
        stream
            .set_read_timeout(Some(Duration::from_secs(
                TIMEOUT_SECONDS,
            )))
            .map_err(map_timeout_error)?;

        let mut buf_reader = BufReader::new(stream);
        let mut request_line = String::new();

        let _ = buf_reader
            .read_line(&mut request_line)
            .map_err(map_read_error)?;

        // Trim the trailing \r\n before checking the length
        let trimmed_request_line = request_line.trim_end();

        // Check if the request line exceeds the maximum allowed length
        if request_line.len() > MAX_REQUEST_LINE_LENGTH {
            return Err(ServerError::invalid_request(format!(
                "Request line too long: {} characters (max {})",
                request_line.len(),
                MAX_REQUEST_LINE_LENGTH
            )));
        }

        let mut parts = trimmed_request_line.split_whitespace();
        let Some(method_part) = parts.next() else {
            return Err(ServerError::invalid_request(
                "Invalid request line: missing method",
            ));
        };
        let Some(path_part) = parts.next() else {
            return Err(ServerError::invalid_request(
                "Invalid request line: missing path",
            ));
        };
        let Some(version_part) = parts.next() else {
            return Err(ServerError::invalid_request(
                "Invalid request line: missing HTTP version",
            ));
        };
        if parts.next().is_some() {
            return Err(ServerError::invalid_request(format!(
                "Invalid request line: expected {} parts",
                REQUEST_PARTS
            )));
        }

        let method = method_part.to_string();
        if !Self::is_valid_method(&method) {
            return Err(ServerError::invalid_request(format!(
                "Invalid HTTP method: {}",
                method
            )));
        }

        let path = path_part.to_string();
        let is_options_asterisk =
            method.eq_ignore_ascii_case("OPTIONS") && path == "*";
        if !path.starts_with('/') && !is_options_asterisk {
            return Err(ServerError::invalid_request(
                "Invalid path: must start with '/' (or be '*' for OPTIONS)",
            ));
        }

        let version = version_part.to_string();
        if !Self::is_valid_version(&version) {
            return Err(ServerError::invalid_request(format!(
                "Invalid HTTP version: {}",
                version
            )));
        }

        let headers = Self::read_headers(&mut buf_reader)?;

        Ok(Request {
            method,
            path,
            version,
            headers,
        })
    }

    /// Returns the HTTP method of the request.
    ///
    /// # Returns
    ///
    /// A string slice containing the HTTP method (e.g., "GET", "POST").
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Returns the requested path of the request.
    ///
    /// # Returns
    ///
    /// A string slice containing the requested path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the HTTP version of the request.
    ///
    /// # Returns
    ///
    /// A string slice containing the HTTP version (e.g., "HTTP/1.1").
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the value of a header by case-insensitive name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::request::Request;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HashMap::new();
    /// headers.insert("content-type".to_string(), "text/plain".to_string());
    /// let request = Request {
    ///     method: "GET".to_string(),
    ///     path: "/".to_string(),
    ///     version: "HTTP/1.1".to_string(),
    ///     headers,
    /// };
    /// assert_eq!(request.header("Content-Type"), Some("text/plain"));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "header lookup")]
    pub fn header(&self, name: &str) -> Option<&str> {
        // Linear scan: header counts in real traffic are O(10), so a
        // case-insensitive equality check beats hashing the lookup key.
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Returns all parsed headers.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Checks if the given method is a valid HTTP method.
    ///
    /// # Arguments
    ///
    /// * `method` - A string slice containing the HTTP method to validate.
    ///
    /// # Returns
    ///
    /// `true` if the method is valid, `false` otherwise.
    fn is_valid_method(method: &str) -> bool {
        matches!(
            method.to_ascii_uppercase().as_str(),
            "GET"
                | "POST"
                | "PUT"
                | "DELETE"
                | "HEAD"
                | "OPTIONS"
                | "PATCH"
        )
    }

    /// Checks if the given HTTP version is supported.
    ///
    /// # Arguments
    ///
    /// * `version` - A string slice containing the HTTP version to validate.
    ///
    /// # Returns
    ///
    /// `true` if the version is supported, `false` otherwise.
    fn is_valid_version(version: &str) -> bool {
        version.eq_ignore_ascii_case("HTTP/1.0")
            || version.eq_ignore_ascii_case("HTTP/1.1")
    }

    fn read_headers<R: BufRead>(
        reader: &mut R,
    ) -> Result<Vec<(String, String)>, ServerError> {
        let mut headers: Vec<(String, String)> = Vec::with_capacity(16);
        let mut total_bytes = 0_usize;
        // Reuse a single line buffer across iterations to avoid allocating
        // a fresh String per header line.
        let mut line = String::new();

        loop {
            line.clear();
            let bytes =
                reader.read_line(&mut line).map_err(map_read_error)?;
            if bytes == 0 {
                break;
            }
            total_bytes = total_bytes.saturating_add(bytes);
            if total_bytes > MAX_HEADER_BYTES {
                return Err(ServerError::invalid_request(
                    "Header section too large",
                ));
            }

            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if trimmed.len() > MAX_HEADER_LINE_LENGTH {
                return Err(ServerError::invalid_request(
                    "Header line too long",
                ));
            }
            // memchr finds the first ':' via SIMD (NEON on Apple
            // Silicon, AVX2 on x86_64). For typical 12–40 byte header
            // lines the win is small; for longer lines (cookies,
            // user-agent) it's measurable.
            let bytes = trimmed.as_bytes();
            let colon =
                memchr::memchr(b':', bytes).ok_or_else(|| {
                    ServerError::invalid_request(
                        "Malformed header line",
                    )
                })?;
            // SAFETY: `colon` is an index returned by memchr inside
            // `bytes`, which is the byte view of the `&str` `trimmed`.
            // ASCII ':' is exactly one UTF-8 byte, so the split lands
            // on a UTF-8 boundary.
            let (name, value) = trimmed.split_at(colon);
            let value = &value[1..];
            if headers.len() >= MAX_HEADER_COUNT {
                return Err(ServerError::invalid_request(
                    "Too many request headers",
                ));
            }
            headers.push((
                name.trim().to_ascii_lowercase(),
                value.trim().to_string(),
            ));
        }

        Ok(headers)
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.method, self.path, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    #[test]
    fn test_valid_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"GET /index.html HTTP/1.1\r\n").unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let request = Request::from_stream(&stream).unwrap();

        assert_eq!(request.method(), "GET");
        assert_eq!(request.path(), "/index.html");
        assert_eq!(request.version(), "HTTP/1.1");
        assert!(request.headers().is_empty());
    }

    #[test]
    fn test_invalid_method() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"INVALID /index.html HTTP/1.1\r\n")
                .unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let result = Request::from_stream(&stream);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServerError::InvalidRequest(_)
        ));
    }

    #[test]
    fn test_max_length_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let long_path = "/".repeat(MAX_REQUEST_LINE_LENGTH - 16); // Account for "GET ", " HTTP/1.1", and "\r\n"
            let request = format!("GET {} HTTP/1.1\r\n", long_path);
            stream.write_all(request.as_bytes()).unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let result = Request::from_stream(&stream);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().path().len(),
            MAX_REQUEST_LINE_LENGTH - 16
        );
    }

    #[test]
    fn test_oversized_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let long_path = "/".repeat(MAX_REQUEST_LINE_LENGTH - 13); // 13 = len("GET  HTTP/1.1")
            let request = format!("GET {} HTTP/1.1\r\n", long_path);
            stream.write_all(request.as_bytes()).unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let result = Request::from_stream(&stream);

        assert!(
            result.is_err(),
            "Oversized request should be invalid. Request: {:?}",
            result
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Request line too long:"),
            "Unexpected error message: {}",
            msg
        );
    }

    #[test]
    fn test_invalid_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"GET index.html HTTP/1.1\r\n").unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let result = Request::from_stream(&stream);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServerError::InvalidRequest(_)
        ));
    }

    #[test]
    fn test_invalid_version() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"GET /index.html HTTP/2.0\r\n").unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let result = Request::from_stream(&stream);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServerError::InvalidRequest(_)
        ));
    }

    #[test]
    fn test_head_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"HEAD /index.html HTTP/1.1\r\n").unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let request = Request::from_stream(&stream).unwrap();

        assert_eq!(request.method(), "HEAD");
        assert_eq!(request.path(), "/index.html");
        assert_eq!(request.version(), "HTTP/1.1");
    }

    #[test]
    fn test_options_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"OPTIONS * HTTP/1.1\r\n").unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let request = Request::from_stream(&stream).unwrap();

        assert_eq!(request.method(), "OPTIONS");
        assert_eq!(request.path(), "*");
        assert_eq!(request.version(), "HTTP/1.1");
    }

    #[test]
    fn test_internal_error_mapping_helpers() {
        let timeout_err =
            io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let mapped = map_timeout_error(timeout_err);
        assert!(
            mapped.to_string().contains("Failed to set read timeout")
        );

        let read_err =
            io::Error::new(io::ErrorKind::UnexpectedEof, "eof");
        let mapped = map_read_error(read_err);
        assert!(
            mapped.to_string().contains("Failed to read request line")
        );
    }

    #[test]
    fn test_parses_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"GET /index.html HTTP/1.1\r\nHost: localhost\r\nRange: bytes=0-1\r\n\r\n",
                )
                .unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let request = Request::from_stream(&stream).unwrap();
        assert_eq!(request.header("host"), Some("localhost"));
        assert_eq!(request.header("range"), Some("bytes=0-1"));
    }

    fn run_request_bytes(
        bytes: Vec<u8>,
    ) -> Result<Request, ServerError> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let _ = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.write_all(&bytes);
        });
        let stream = TcpStream::connect(addr).unwrap();
        Request::from_stream(&stream)
    }

    #[test]
    fn test_missing_method_returns_error() {
        let err = run_request_bytes(b"\r\n".to_vec()).unwrap_err();
        assert!(
            err.to_string().contains("missing method"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_too_many_parts_returns_error() {
        let err =
            run_request_bytes(b"GET / HTTP/1.1 extra\r\n".to_vec())
                .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("expected") && msg.contains("parts"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_malformed_header_returns_error() {
        let err = run_request_bytes(
            b"GET / HTTP/1.1\r\nmissing-colon-line\r\n\r\n".to_vec(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("Malformed header line"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_header_line_too_long_returns_error() {
        let mut req = Vec::from("GET / HTTP/1.1\r\nX: ");
        req.extend(
            std::iter::repeat(b'A').take(MAX_HEADER_LINE_LENGTH),
        );
        req.extend_from_slice(b"\r\n\r\n");
        let err = run_request_bytes(req).unwrap_err();
        assert!(
            err.to_string().contains("Header line too long"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_header_section_too_large_returns_error() {
        // Many moderately sized header lines (each under MAX_HEADER_LINE_LENGTH)
        // whose cumulative byte count exceeds MAX_HEADER_BYTES before the
        // per-line or header-count guards trip.
        let mut req = Vec::from("GET / HTTP/1.1\r\n");
        let filler: String = "A".repeat(8000);
        // Ten ~8KiB headers = ~80 KiB > 64 KiB cap.
        for i in 0..10 {
            req.extend_from_slice(
                format!("H{i}: {filler}\r\n").as_bytes(),
            );
        }
        req.extend_from_slice(b"\r\n");
        let err = run_request_bytes(req).unwrap_err();
        assert!(
            err.to_string().contains("Header section too large"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_too_many_headers_returns_error() {
        let mut req = Vec::from("GET / HTTP/1.1\r\n");
        for i in 0..=MAX_HEADER_COUNT {
            req.extend_from_slice(format!("H{i}: v\r\n").as_bytes());
        }
        req.extend_from_slice(b"\r\n");
        let err = run_request_bytes(req).unwrap_err();
        assert!(
            err.to_string().contains("Too many request headers"),
            "unexpected error: {err}"
        );
    }
}
