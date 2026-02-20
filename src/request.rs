// src/request.rs

//! HTTP request parsing module for the Http Handle.
//!
//! This module provides functionality to parse incoming HTTP requests from a TCP stream.
//! It defines the `Request` struct and associated methods for creating and interacting with HTTP requests in a secure and robust manner.

use crate::error::ServerError;
use std::collections::HashMap;
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

/// Represents an HTTP request, containing the HTTP method, the requested path, and the HTTP version.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// HTTP method of the request.
    pub method: String,
    /// Requested path.
    pub path: String,
    /// HTTP version of the request.
    pub version: String,
    /// Parsed request headers (header-name lowercased).
    pub headers: HashMap<String, String>,
}

impl Request {
    /// Attempts to create a `Request` from the provided TCP stream by reading the first line.
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
    /// ```
    /// use std::net::TcpStream;
    /// use http_handle::request::Request;
    ///
    /// fn handle_client(stream: TcpStream) {
    ///     match Request::from_stream(&stream) {
    ///         Ok(request) => println!("Received request: {}", request),
    ///         Err(e) => eprintln!("Error parsing request: {}", e),
    ///     }
    /// }
    /// ```
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
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// Returns all parsed headers.
    pub fn headers(&self) -> &HashMap<String, String> {
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
    ) -> Result<HashMap<String, String>, ServerError> {
        let mut headers = HashMap::with_capacity(16);
        let mut total_bytes = 0_usize;

        loop {
            let mut line = String::new();
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
            let (name, value) =
                trimmed.split_once(':').ok_or_else(|| {
                    ServerError::invalid_request(
                        "Malformed header line",
                    )
                })?;
            if headers.len() >= MAX_HEADER_COUNT {
                return Err(ServerError::invalid_request(
                    "Too many request headers",
                ));
            }
            let _ = headers.insert(
                name.trim().to_ascii_lowercase(),
                value.trim().to_string(),
            );
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
}
