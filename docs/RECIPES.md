# Recipes

Practical snippets for common `http-handle` integration patterns.

## Serve a Static Site with Custom Security Header

```rust,no_run
use http_handle::Server;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let server = Server::builder()
        .address("127.0.0.1:8080")
        .document_root("./public")
        .request_timeout(Duration::from_secs(20))
        .add_header("X-Content-Type-Options", "nosniff")
        .build()?;

    server.start()
}
```

## Enable Async + High Performance Path

```bash
HTTP_HANDLE_MODE=high-perf \
HTTP_HANDLE_ADDR=127.0.0.1:8090 \
HTTP_HANDLE_ROOT=./public \
cargo run --example benchmark_target --features async,high-perf
```

## Validate Feature-Scoped Example

```bash
cargo run --example feature_http2_server --features http2
```

## Release Readiness Recipe

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --examples --all-features
./scripts/score_docs.sh
./scripts/enforce_docs_governance.sh
```
