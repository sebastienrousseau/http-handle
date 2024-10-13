//! Error types for the Http Handle.
//!
//! This module defines the various error types that can occur during the operation
//! of the Http Handle. It provides a centralized place for error handling and
//! propagation throughout the application.
//!
//! The main type exposed by this module is the `ServerError` enum, which
//! encompasses all possible error conditions the server might encounter.

use std::io;
use thiserror::Error;

/// Represents the different types of errors that can occur in the server.
///
/// This enum defines various errors that can be encountered during the server's operation,
/// such as I/O errors, invalid requests, file not found, and forbidden access.
///
/// # Examples
///
/// Creating an I/O error:
///
/// ```
/// use std::io::{Error, ErrorKind};
/// use http_handle::ServerError;
///
/// let io_error = Error::new(ErrorKind::NotFound, "file not found");
/// let server_error = ServerError::from(io_error);
/// assert!(matches!(server_error, ServerError::Io(_)));
/// ```
///
/// Creating an invalid request error:
///
/// ```
/// use http_handle::ServerError;
///
/// let invalid_request = ServerError::InvalidRequest("Missing HTTP method".to_string());
/// assert!(matches!(invalid_request, ServerError::InvalidRequest(_)));
/// ```
#[derive(Error, Debug)]
pub enum ServerError {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// The request received by the server was invalid or malformed.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The requested file was not found on the server.
    #[error("File not found: {0}")]
    NotFound(String),

    /// Access to the requested resource is forbidden.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// A custom error type for unexpected scenarios.
    #[error("Custom error: {0}")]
    Custom(String),
}

impl ServerError {
    /// Creates a new `InvalidRequest` error with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - A string slice that holds the error message.
    ///
    /// # Returns
    ///
    /// A `ServerError::InvalidRequest` variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use http_handle::ServerError;
    ///
    /// let error = ServerError::invalid_request("Missing HTTP version");
    /// assert!(matches!(error, ServerError::InvalidRequest(_)));
    /// ```
    pub fn invalid_request<T: Into<String>>(message: T) -> Self {
        ServerError::InvalidRequest(message.into())
    }

    /// Creates a new `NotFound` error with the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that holds the path of the not found resource.
    ///
    /// # Returns
    ///
    /// A `ServerError::NotFound` variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use http_handle::ServerError;
    ///
    /// let error = ServerError::not_found("/nonexistent.html");
    /// assert!(matches!(error, ServerError::NotFound(_)));
    /// ```
    pub fn not_found<T: Into<String>>(path: T) -> Self {
        ServerError::NotFound(path.into())
    }

    /// Creates a new `Forbidden` error with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - A string slice that holds the error message.
    ///
    /// # Returns
    ///
    /// A `ServerError::Forbidden` variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use http_handle::ServerError;
    ///
    /// let error = ServerError::forbidden("Access denied to sensitive file");
    /// assert!(matches!(error, ServerError::Forbidden(_)));
    /// ```
    pub fn forbidden<T: Into<String>>(message: T) -> Self {
        ServerError::Forbidden(message.into())
    }
}

impl From<&str> for ServerError {
    /// Converts a string slice into a `ServerError::Custom` variant.
    ///
    /// This implementation allows for easy creation of custom errors from string literals.
    ///
    /// # Arguments
    ///
    /// * `error` - A string slice that holds the error message.
    ///
    /// # Returns
    ///
    /// A `ServerError::Custom` variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use http_handle::ServerError;
    ///
    /// let error: ServerError = "Unexpected error".into();
    /// assert!(matches!(error, ServerError::Custom(_)));
    /// ```
    fn from(error: &str) -> Self {
        ServerError::Custom(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_error =
            io::Error::new(io::ErrorKind::NotFound, "file not found");
        let server_error = ServerError::from(io_error);
        assert!(matches!(server_error, ServerError::Io(_)));
    }

    #[test]
    fn test_custom_error_creation() {
        let custom_error = ServerError::from("Unexpected error");
        assert!(matches!(custom_error, ServerError::Custom(_)));
    }

    #[test]
    fn test_error_messages() {
        let not_found = ServerError::not_found("index.html");
        assert_eq!(not_found.to_string(), "File not found: index.html");

        let forbidden = ServerError::forbidden("Access denied");
        assert_eq!(forbidden.to_string(), "Forbidden: Access denied");

        let invalid_request =
            ServerError::invalid_request("Missing HTTP method");
        assert_eq!(
            invalid_request.to_string(),
            "Invalid request: Missing HTTP method"
        );
    }

    #[test]
    fn test_error_creation_methods() {
        let invalid_request =
            ServerError::invalid_request("Bad request");
        assert!(matches!(
            invalid_request,
            ServerError::InvalidRequest(_)
        ));

        let not_found = ServerError::not_found("/nonexistent.html");
        assert!(matches!(not_found, ServerError::NotFound(_)));

        let forbidden = ServerError::forbidden("Access denied");
        assert!(matches!(forbidden, ServerError::Forbidden(_)));
    }
}
