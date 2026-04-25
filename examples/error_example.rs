// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

#![allow(missing_docs)]

use http_handle::ServerError;

/// Entry point for the Http Handle error handling examples.
///
/// This function runs various examples demonstrating error creation, conversion,
/// and handling for different scenarios in the Http Handle library.
///
/// # Errors
///
/// Returns an error if any of the example functions fail.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Http Handle Error Handling Examples\n");
    io_error_example()?;
    invalid_request_error_example()?;
    not_found_error_example()?;
    forbidden_error_example()?;
    custom_error_example()?;
    println!(
        "\n🎉 All error handling examples completed successfully!"
    );
    Ok(())
}

/// Demonstrates handling of I/O errors.
fn io_error_example() -> Result<(), ServerError> {
    println!("🦀 I/O Error Example");
    println!("---------------------------------------------");
    let io_error = std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found",
    );
    let server_error = ServerError::from(io_error);
    println!("    ✅ Created I/O Error: {}", server_error);
    Ok(())
}

/// Demonstrates handling of invalid request errors.
fn invalid_request_error_example() -> Result<(), ServerError> {
    println!("\n🦀 Invalid Request Error Example");
    println!("---------------------------------------------");
    let error = ServerError::invalid_request("Missing HTTP method");
    println!("    ✅ Created Invalid Request Error: {}", error);
    Ok(())
}

/// Demonstrates handling of file not found errors.
fn not_found_error_example() -> Result<(), ServerError> {
    println!("\n🦀 File Not Found Error Example");
    println!("---------------------------------------------");
    let error = ServerError::not_found("/nonexistent.html");
    println!("    ✅ Created Not Found Error: {}", error);
    Ok(())
}

/// Demonstrates handling of forbidden access errors.
fn forbidden_error_example() -> Result<(), ServerError> {
    println!("\n🦀 Forbidden Access Error Example");
    println!("---------------------------------------------");
    let error =
        ServerError::forbidden("Access denied to sensitive file");
    println!("    ✅ Created Forbidden Error: {}", error);
    Ok(())
}

/// Demonstrates handling of custom errors.
fn custom_error_example() -> Result<(), ServerError> {
    println!("\n🦀 Custom Error Example");
    println!("---------------------------------------------");
    let error: ServerError = "Unexpected error".into();
    println!("    ✅ Created Custom Error: {}", error);
    Ok(())
}
