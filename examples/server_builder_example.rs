//! Example demonstrating the ServerBuilder pattern usage
//!
//! This example shows how to use the ServerBuilder to configure a server
//! with optional features like CORS, custom headers, and timeouts.

use http_handle::Server;
use std::collections::HashMap;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 HTTP Handle ServerBuilder Example");
    println!("=====================================");
    demonstrate_builder_benefits();

    // Example 1: Basic server using traditional constructor
    println!("\n📦 Example 1: Traditional Server Constructor");
    let basic_server = Server::new("127.0.0.1:8080", "examples");
    println!("✅ Basic server created: 127.0.0.1:8080");

    // Example 2: Server with CORS enabled using ServerBuilder
    println!("\n🌐 Example 2: Server with CORS Configuration");
    let _cors_server = Server::builder()
        .address("127.0.0.1:8081")
        .document_root("examples")
        .enable_cors()
        .cors_origins(vec![
            "https://localhost:3000".to_string(),
            "https://example.com".to_string(),
            "https://api.example.com".to_string(),
        ])
        .build()?;
    println!("✅ CORS-enabled server created on port 8081");

    // Example 3: Server with custom headers
    println!("\n🏷️  Example 3: Server with Custom Headers");
    let _header_server = Server::builder()
        .address("127.0.0.1:8082")
        .document_root("examples")
        .custom_header("X-Powered-By", "http-handle v0.0.3")
        .custom_header("X-Content-Type-Options", "nosniff")
        .custom_header("X-Frame-Options", "DENY")
        .custom_header("X-XSS-Protection", "1; mode=block")
        .build()?;
    println!("✅ Server with security headers created on port 8082");

    // Example 4: Server with custom timeouts
    println!("\n⏱️  Example 4: Server with Custom Timeouts");
    let _timeout_server = Server::builder()
        .address("127.0.0.1:8083")
        .document_root("examples")
        .request_timeout(Duration::from_secs(30))
        .connection_timeout(Duration::from_secs(60))
        .build()?;
    println!("✅ Server with custom timeouts created on port 8083");

    // Example 5: Server with bulk custom headers using HashMap
    println!("\n📋 Example 5: Server with Bulk Headers Configuration");
    let mut headers = HashMap::new();
    let _ =
        headers.insert("X-Api-Version".to_string(), "v1.0".to_string());
    let _ =
        headers.insert("X-Rate-Limit".to_string(), "1000".to_string());
    let _ = headers
        .insert("X-Rate-Limit-Window".to_string(), "3600".to_string());

    let _bulk_header_server = Server::builder()
        .address("127.0.0.1:8084")
        .document_root("examples")
        .custom_headers(headers)
        .build()?;
    println!("✅ Server with bulk headers created on port 8084");

    // Example 6: Fully configured server with all optional features
    println!("\n🎯 Example 6: Fully Configured Server");
    let _full_server = Server::builder()
        .address("127.0.0.1:8085")
        .document_root("examples")
        .enable_cors()
        .cors_origins(vec!["*".to_string()])
        .custom_header("X-Server-Name", "http-handle-demo")
        .custom_header("X-Version", "0.0.3")
        .request_timeout(Duration::from_secs(45))
        .connection_timeout(Duration::from_secs(120))
        .build()?;
    println!("✅ Fully configured server created on port 8085");

    // Example 7: Demonstrate CORS enable/disable toggle
    println!("\n🔄 Example 7: CORS Toggle Demonstration");
    let _toggled_server = Server::builder()
        .address("127.0.0.1:8086")
        .document_root("examples")
        .enable_cors()
        .cors_origins(vec!["https://localhost".to_string()])
        .disable_cors() // Disable CORS after enabling
        .build()?;
    println!(
        "✅ Server with toggled CORS (disabled) created on port 8086"
    );

    // Example 8: Error handling demonstration
    println!("\n⚠️  Example 8: Error Handling");
    let invalid_server_result = Server::builder()
        .address("127.0.0.1:8087")
        // Missing document_root - should fail
        .build();

    match invalid_server_result {
        Ok(_) => println!(
            "❌ This should not happen - server should fail without document_root"
        ),
        Err(e) => println!("✅ Correctly caught error: {}", e),
    }

    // Example 9: Start one of the servers for demonstration
    println!(
        "\n🏃 Example 9: Starting a Server with Graceful Shutdown"
    );
    println!("Starting server on http://127.0.0.1:8080...");
    println!("Press Ctrl+C to stop the server gracefully.");

    // Start the basic server with graceful shutdown
    match basic_server
        .start_with_graceful_shutdown(Duration::from_secs(30))
    {
        Ok(_) => println!("✅ Server stopped gracefully"),
        Err(e) => eprintln!("❌ Server error: {}", e),
    }

    Ok(())
}

// Example of how the ServerBuilder pattern improves configuration
fn demonstrate_builder_benefits() {
    println!("\n💡 ServerBuilder Pattern Benefits:");
    println!("==================================");

    // Traditional approach (limited flexibility)
    println!("\n📜 Traditional Approach:");
    println!(
        "let server = Server::new(\"127.0.0.1:8080\", \"public\");"
    );
    println!("// No way to configure CORS, headers, or timeouts");

    // Builder pattern approach (highly flexible)
    println!("\n🔧 Builder Pattern Approach:");
    println!("let server = Server::builder()");
    println!("    .address(\"127.0.0.1:8080\")");
    println!("    .document_root(\"public\")");
    println!("    .enable_cors()");
    println!(
        "    .cors_origins(vec![\"https://myapp.com\".to_string()])"
    );
    println!("    .custom_header(\"X-Powered-By\", \"my-app\")");
    println!("    .request_timeout(Duration::from_secs(30))");
    println!("    .build()?;");

    println!("\n✨ Benefits:");
    println!("  ✓ Backward compatible (Server::new still works)");
    println!("  ✓ Optional configuration only when needed");
    println!("  ✓ Fluent, readable API");
    println!("  ✓ Compile-time validation of required fields");
    println!("  ✓ Easy to extend with new configuration options");
}
