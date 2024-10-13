// src/request.rs

//! HTTP request parsing module for the Http Handle.
//!
//! This module provides functionality to parse incoming HTTP requests from a TCP stream.
//! It defines the `Request` struct and associated methods for creating and interacting with HTTP requests in a secure and robust manner.

use crate::error::ServerError;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::time::Duration;

/// Maximum length allowed for the request line (8KB).
/// This includes the method, path, version, and the two spaces between them, but not the trailing \r\n.
const MAX_REQUEST_LINE_LENGTH: usize = 8190;

/// Number of parts expected in a valid HTTP request line.
const REQUEST_PARTS: usize = 3;

/// Timeout duration for reading from the TCP stream (in seconds).
const TIMEOUT_SECONDS: u64 = 30;

/// Represents an HTTP request, containing the HTTP method, the requested path, and the HTTP version.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// HTTP method of the request.
    pub method: String,
    /// Requested path.
    pub path: String,
    /// HTTP version of the request.
    pub version: String,
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
    /// - The request path does not start with a forward slash
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
            .map_err(|e| {
                ServerError::invalid_request(format!(
                    "Failed to set read timeout: {}",
                    e
                ))
            })?;

        let mut buf_reader = BufReader::new(stream);
        let mut request_line = String::new();

        let _ =
            buf_reader.read_line(&mut request_line).map_err(|e| {
                ServerError::invalid_request(format!(
                    "Failed to read request line: {}",
                    e
                ))
            })?;

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

        let parts: Vec<&str> =
            trimmed_request_line.split_whitespace().collect();

        if parts.len() != REQUEST_PARTS {
            return Err(ServerError::invalid_request(format!(
                "Invalid request line: expected {} parts, got {}",
                REQUEST_PARTS,
                parts.len()
            )));
        }

        let method = parts[0].to_string();
        if !Self::is_valid_method(&method) {
            return Err(ServerError::invalid_request(format!(
                "Invalid HTTP method: {}",
                method
            )));
        }

        let path = parts[1].to_string();
        if !path.starts_with('/') {
            return Err(ServerError::invalid_request(
                "Invalid path: must start with '/'",
            ));
        }

        let version = parts[2].to_string();
        if !Self::is_valid_version(&version) {
            return Err(ServerError::invalid_request(format!(
                "Invalid HTTP version: {}",
                version
            )));
        }

        Ok(Request {
            method,
            path,
            version,
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

        assert!(
            result.is_ok(),
            "Max length request should be valid. Error: {:?}",
            result.err()
        );
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
        match result.unwrap_err() {
            ServerError::InvalidRequest(msg) => {
                assert!(
                    msg.starts_with("Request line too long:"),
                    "Unexpected error message: {}",
                    msg
                );
            }
            _ => panic!("Unexpected error type"),
        }
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
}
