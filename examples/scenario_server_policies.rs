//! Server policy configuration scenario.

use http_handle::Server;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::builder()
        .address("127.0.0.1:8080")
        .document_root("./public")
        .enable_cors()
        .cors_origins(vec!["https://example.com".to_string()])
        .custom_header("X-Frame-Options", "DENY")
        .custom_header("X-Content-Type-Options", "nosniff")
        .request_timeout(Duration::from_secs(15))
        .connection_timeout(Duration::from_secs(30))
        .rate_limit_per_minute(120)
        .static_cache_ttl_secs(300)
        .build()?;

    println!("configured server address: {}", server.address());
    println!("configured root: {}", server.document_root().display());
    Ok(())
}
