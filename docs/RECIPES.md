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
cargo run --release --example bench --features 'async,high-perf,high-perf-multi-thread,http2'
```

For the `high-perf-mt` mode (multi-thread Tokio runtime, the
throughput leader on every Linux row in `docs/PERFORMANCE.md`):

```bash
HTTP_HANDLE_MODE=high-perf-mt \
HTTP_HANDLE_WORKERS=4 \
cargo run --release --example bench --features 'high-perf-multi-thread'
```

## Validate Feature-Scoped Example

```bash
./scripts/example.sh http2     # auto-resolves --features http2
./scripts/example.sh enterprise
./scripts/example.sh --list    # 30 demos, one per feature
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
