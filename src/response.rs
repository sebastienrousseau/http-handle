// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

// src/response.rs

//! HTTP response construction and serialization.
//!
//! Use this module to build status lines, headers, and body payloads and emit them to any
//! writable stream with stable HTTP/1.1 framing defaults.

use crate::error::ServerError;
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};

/// Represents an HTTP response payload and metadata.
///
/// You create this type on the response path, add headers, and serialize it to any
/// `Write` sink (for example `TcpStream` or an in-memory buffer in tests).
///
/// # Examples
///
/// ```rust
/// use http_handle::response::Response;
///
/// let response = Response::new(200, "OK", b"hello".to_vec());
/// assert_eq!(response.status_code, 200);
/// ```
///
/// # Panics
///
/// This type does not panic on construction.
#[doc(alias = "http response")]
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize,
)]
pub struct Response {
    /// The HTTP status code (e.g., 200 for OK, 404 for Not Found).
    pub status_code: u16,

    /// The HTTP status text associated with the status code (e.g., "OK", "Not Found").
    pub status_text: String,

    /// A list of headers in the response, each represented as a tuple containing the header
    /// name and its corresponding value.
    pub headers: Vec<(String, String)>,

    /// The body of the response, represented as a vector of bytes.
    pub body: Vec<u8>,
}

impl Response {
    /// Creates a response with status, reason, and body bytes.
    ///
    /// The headers are initialized as an empty list and can be added later using the `add_header` method.
    ///
    /// # Arguments
    ///
    /// * `status_code` - The HTTP status code for the response.
    /// * `status_text` - The status text corresponding to the status code.
    /// * `body` - The body of the response, represented as a vector of bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::response::Response;
    ///
    /// let response = Response::new(204, "NO CONTENT", Vec::new());
    /// assert_eq!(response.status_code, 204);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "constructor")]
    pub fn new(
        status_code: u16,
        status_text: &str,
        body: Vec<u8>,
    ) -> Self {
        Response {
            status_code,
            status_text: status_text.to_string(),
            headers: Vec::new(),
            body,
        }
    }

    /// Adds a header to the response.
    ///
    /// This method allows you to add custom headers to the response, which will be included
    /// in the HTTP response when it is sent to the client.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::response::Response;
    ///
    /// let mut response = Response::new(200, "OK", Vec::new());
    /// response.add_header("Content-Type", "text/plain");
    /// assert_eq!(response.headers.len(), 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "set header")]
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(), value.to_string()));
    }

    /// Sends the response over the provided `Write` stream.
    ///
    /// This method writes the HTTP status line, headers, and body to the stream, ensuring
    /// the client receives the complete response.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to any stream that implements `Write`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::response::Response;
    /// use std::io::Cursor;
    ///
    /// let mut response = Response::new(200, "OK", b"hello".to_vec());
    /// response.add_header("Content-Type", "text/plain");
    ///
    /// let mut out = Cursor::new(Vec::<u8>::new());
    /// response.send(&mut out).expect("response write should succeed");
    /// assert!(!out.get_ref().is_empty());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` when writing headers or body to the output stream fails.
    ///
    /// # Panics
    ///
    /// This function does not intentionally panic.
    #[doc(alias = "serialize")]
    #[doc(alias = "write response")]
    pub fn send<W: Write>(
        &self,
        stream: &mut W,
    ) -> Result<(), ServerError> {
        // Coalesce status line, headers, and trailer CRLF into a single
        // buffered flush. Prior implementation emitted one write() syscall
        // per header field; for a typical 5-header response that collapses
        // 8+ syscalls into 1–2.
        let mut w = BufWriter::with_capacity(4096, stream);

        let mut has_content_length = false;
        let mut has_connection = false;

        write!(
            w,
            "HTTP/1.1 {} {}\r\n",
            self.status_code, self.status_text
        )?;

        for (name, value) in &self.headers {
            if name.eq_ignore_ascii_case("content-length") {
                has_content_length = true;
            }
            if name.eq_ignore_ascii_case("connection") {
                has_connection = true;
            }
            write!(w, "{}: {}\r\n", name, value)?;
        }

        if !has_content_length {
            write!(w, "Content-Length: {}\r\n", self.body.len())?;
        }
        if !has_connection {
            w.write_all(b"Connection: close\r\n")?;
        }

        w.write_all(b"\r\n")?;
        w.write_all(&self.body)?;
        w.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor, Write};

    /// Test case for the `Response::new` method.
    #[test]
    fn test_response_new() {
        let status_code = 200;
        let status_text = "OK";
        let body = b"Hello, world!".to_vec();
        let response =
            Response::new(status_code, status_text, body.clone());

        assert_eq!(response.status_code, status_code);
        assert_eq!(response.status_text, status_text.to_string());
        assert!(response.headers.is_empty());
        assert_eq!(response.body, body);
    }

    /// Test case for the `Response::add_header` method.
    #[test]
    fn test_response_add_header() {
        let mut response = Response::new(200, "OK", vec![]);
        response.add_header("Content-Type", "text/html");

        assert_eq!(response.headers.len(), 1);
        assert_eq!(
            response.headers[0],
            ("Content-Type".to_string(), "text/html".to_string())
        );
    }

    /// A mock implementation of `Write` to simulate writing the response without actual network operations.
    struct MockTcpStream {
        buffer: Cursor<Vec<u8>>,
    }

    impl MockTcpStream {
        fn new() -> Self {
            MockTcpStream {
                buffer: Cursor::new(Vec::new()),
            }
        }

        fn get_written_data(&self) -> Vec<u8> {
            self.buffer.clone().into_inner()
        }
    }

    impl Write for MockTcpStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buffer.flush()
        }
    }

    /// Test case for the `Response::send` method.
    #[test]
    fn test_response_send() {
        let mut response =
            Response::new(200, "OK", b"Hello, world!".to_vec());
        response.add_header("Content-Type", "text/plain");

        let mut mock_stream = MockTcpStream::new();
        let result = response.send(&mut mock_stream);

        assert!(result.is_ok());

        let expected_output = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\nConnection: close\r\n\r\nHello, world!";
        let written_data = mock_stream.get_written_data();

        assert_eq!(written_data, expected_output);
    }

    /// Test case for `Response::send` when there is an error during writing.
    #[test]
    fn test_response_send_error() {
        let mut response =
            Response::new(200, "OK", b"Hello, world!".to_vec());
        response.add_header("Content-Type", "text/plain");

        struct FailingStream;

        impl Write for FailingStream {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("write error"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut failing_stream = FailingStream;
        let result = response.send(&mut failing_stream);
        failing_stream.flush().expect("flush");

        assert!(result.is_err());
    }
}
