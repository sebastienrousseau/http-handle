// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

// src/error.rs

//! Error model for runtime, parsing, and policy operations.
//!
//! Use [`ServerError`] as the shared error boundary across sync and async server paths.

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

    /// A task execution failed (join failure or panic boundary).
    #[error("Task failed: {0}")]
    TaskFailed(String),
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

impl From<ServerError> for io::Error {
    /// Converts a `ServerError` into an `io::Error`.
    ///
    /// This implementation enables the `?` operator to convert `ServerError`
    /// to `io::Error` when needed, particularly for functions that return
    /// `io::Result<()>` but work with `ServerError` internally.
    ///
    /// # Arguments
    ///
    /// * `error` - A `ServerError` to convert.
    ///
    /// # Returns
    ///
    /// An `io::Error` with the appropriate error kind and message.
    fn from(error: ServerError) -> Self {
        match error {
            ServerError::Io(io_error) => io_error,
            ServerError::InvalidRequest(msg) => {
                io::Error::new(io::ErrorKind::InvalidInput, msg)
            }
            ServerError::NotFound(msg) => {
                io::Error::new(io::ErrorKind::NotFound, msg)
            }
            ServerError::Forbidden(msg) => {
                io::Error::new(io::ErrorKind::PermissionDenied, msg)
            }
            ServerError::Custom(msg) => io::Error::other(msg),
            ServerError::TaskFailed(msg) => io::Error::other(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    /// Test case for converting an `io::Error` into `ServerError::Io`.
    #[test]
    fn test_io_error_conversion() {
        let io_error =
            io::Error::new(io::ErrorKind::NotFound, "file not found");
        let server_error = ServerError::from(io_error);
        assert!(matches!(server_error, ServerError::Io(_)));
    }

    /// Test case for creating a `ServerError::Custom` from a string slice.
    #[test]
    fn test_custom_error_creation() {
        let custom_error: ServerError = "Unexpected error".into();
        assert!(matches!(custom_error, ServerError::Custom(_)));
    }

    /// Test case for verifying the error messages of different `ServerError` variants.
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

    /// Test case for creating a `ServerError::InvalidRequest` using the `invalid_request` method.
    #[test]
    fn test_invalid_request_creation() {
        let invalid_request =
            ServerError::invalid_request("Bad request");
        assert!(matches!(
            invalid_request,
            ServerError::InvalidRequest(_)
        ));
        assert_eq!(
            invalid_request.to_string(),
            "Invalid request: Bad request"
        );
    }

    /// Test case for creating a `ServerError::NotFound` using the `not_found` method.
    #[test]
    fn test_not_found_creation() {
        let not_found = ServerError::not_found("/nonexistent.html");
        assert!(matches!(not_found, ServerError::NotFound(_)));
        assert_eq!(
            not_found.to_string(),
            "File not found: /nonexistent.html"
        );
    }

    /// Test case for creating a `ServerError::Forbidden` using the `forbidden` method.
    #[test]
    fn test_forbidden_creation() {
        let forbidden = ServerError::forbidden("Access denied");
        assert!(matches!(forbidden, ServerError::Forbidden(_)));
        assert_eq!(forbidden.to_string(), "Forbidden: Access denied");
    }

    /// Test case for verifying the `ServerError::Custom` variant and its error message.
    #[test]
    fn test_custom_error_message() {
        let custom_error =
            ServerError::Custom("Custom error occurred".to_string());
        assert_eq!(
            custom_error.to_string(),
            "Custom error: Custom error occurred"
        );
    }

    /// Test case for checking `ServerError::from` for string conversion.
    #[test]
    fn test_custom_error_from_str() {
        let custom_error: ServerError = "Some custom error".into();
        assert!(matches!(custom_error, ServerError::Custom(_)));
        assert_eq!(
            custom_error.to_string(),
            "Custom error: Some custom error"
        );
    }

    #[test]
    fn test_task_failed_error_message() {
        let task_failed =
            ServerError::TaskFailed("panic in task".to_string());
        assert_eq!(
            task_failed.to_string(),
            "Task failed: panic in task"
        );
    }

    /// Test case for converting `io::Error` using a different error kind to `ServerError::Io`.
    #[test]
    fn test_io_error_conversion_other_kind() {
        let io_error = io::Error::new(
            io::ErrorKind::PermissionDenied,
            "permission denied",
        );
        let server_error = ServerError::from(io_error);
        assert!(matches!(server_error, ServerError::Io(_)));
        assert_eq!(
            server_error.to_string(),
            "I/O error: permission denied"
        );
    }

    /// Test case for verifying if `ServerError::InvalidRequest` carries the correct error message.
    #[test]
    fn test_invalid_request_message() {
        let error_message = "Invalid HTTP version";
        let invalid_request =
            ServerError::InvalidRequest(error_message.to_string());
        assert_eq!(
            invalid_request.to_string(),
            format!("Invalid request: {}", error_message)
        );
    }

    /// Test case for verifying if `ServerError::NotFound` carries the correct file path.
    #[test]
    fn test_not_found_message() {
        let file_path = "missing.html";
        let not_found = ServerError::NotFound(file_path.to_string());
        assert_eq!(
            not_found.to_string(),
            format!("File not found: {}", file_path)
        );
    }

    /// Test case for verifying if `ServerError::Forbidden` carries the correct message.
    #[test]
    fn test_forbidden_message() {
        let forbidden_message = "Access denied to private resource";
        let forbidden =
            ServerError::Forbidden(forbidden_message.to_string());
        assert_eq!(
            forbidden.to_string(),
            format!("Forbidden: {}", forbidden_message)
        );
    }

    /// Test case for `ServerError::Io` with a generic IO error to ensure correct propagation.
    #[test]
    fn test_io_error_generic() {
        let io_error = io::Error::other("generic I/O error");
        let server_error = ServerError::from(io_error);
        assert!(matches!(server_error, ServerError::Io(_)));
        assert_eq!(
            server_error.to_string(),
            "I/O error: generic I/O error"
        );
    }

    #[test]
    fn test_server_error_to_io_error_conversion() {
        let converted: io::Error =
            ServerError::invalid_request("invalid").into();
        assert_eq!(converted.kind(), io::ErrorKind::InvalidInput);

        let converted: io::Error = ServerError::not_found("x").into();
        assert_eq!(converted.kind(), io::ErrorKind::NotFound);

        let converted: io::Error = ServerError::forbidden("x").into();
        assert_eq!(converted.kind(), io::ErrorKind::PermissionDenied);

        let converted: io::Error =
            ServerError::Custom("custom".to_string()).into();
        assert_eq!(converted.kind(), io::ErrorKind::Other);

        let converted: io::Error =
            ServerError::TaskFailed("task".to_string()).into();
        assert_eq!(converted.kind(), io::ErrorKind::Other);

        let source = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let converted: io::Error = ServerError::Io(source).into();
        assert_eq!(converted.kind(), io::ErrorKind::TimedOut);
    }
}
