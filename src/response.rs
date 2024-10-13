use crate::error::ServerError;
use serde::{Deserialize, Serialize};
use std::io::Write;

/// Represents an HTTP response, including the status code, status text, headers, and body.
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
    /// Creates a new `Response` with the given status code, status text, and body.
    ///
    /// The headers are initialized as an empty list and can be added later using the `add_header` method.
    ///
    /// # Arguments
    ///
    /// * `status_code` - The HTTP status code for the response.
    /// * `status_text` - The status text corresponding to the status code.
    /// * `body` - The body of the response, represented as a vector of bytes.
    ///
    /// # Returns
    ///
    /// A new `Response` instance with the specified status code, status text, and body.
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
    /// # Arguments
    ///
    /// * `name` - The name of the header (e.g., "Content-Type").
    /// * `value` - The value of the header (e.g., "text/html").
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
    /// # Returns
    ///
    /// * `Ok(())` - If the response is successfully sent.
    /// * `Err(ServerError)` - If an error occurs while sending the response.
    pub fn send<W: Write>(
        &self,
        stream: &mut W,
    ) -> Result<(), ServerError> {
        write!(
            stream,
            "HTTP/1.1 {} {}\r\n",
            self.status_code, self.status_text
        )?;

        for (name, value) in &self.headers {
            write!(stream, "{}: {}\r\n", name, value)?;
        }

        write!(stream, "\r\n")?;
        stream.write_all(&self.body)?;
        stream.flush()?;

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

        let expected_output = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello, world!";
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
                Err(io::Error::new(io::ErrorKind::Other, "write error"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut failing_stream = FailingStream;
        let result = response.send(&mut failing_stream);

        assert!(result.is_err());
    }
}
