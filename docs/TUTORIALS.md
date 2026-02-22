# Tutorials

Use these tutorials to move from local development to production operations.

## 1. Launch Your First Static Server

You want a working server in minutes.

### Steps

1. Create a public directory:

```bash
mkdir -p public
echo '<h1>Hello from http-handle</h1>' > public/index.html
```

2. Add `http-handle` to `Cargo.toml`:

```toml
[dependencies]
http-handle = "0.0.3"
```

3. Start the server:

```rust,no_run
use http_handle::Server;

fn main() -> std::io::Result<()> {
    Server::new("127.0.0.1:8080", "./public").start()
}
```

4. Verify behavior:

```bash
curl -i http://127.0.0.1:8080/
```

Expected result:
- Status `200 OK`
- Content served from `./public/index.html`

## 2. Move to Policy-Driven Configuration

You want explicit timeout, CORS, and header policy.

### Steps

1. Switch to `ServerBuilder`:

```rust,no_run
use http_handle::Server;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let server = Server::builder()
        .address("127.0.0.1:8080")
        .document_root("./public")
        .request_timeout(Duration::from_secs(15))
        .enable_cors(true)
        .add_header("X-Content-Type-Options", "nosniff")
        .build()?;

    server.start()
}
```

2. Verify policy response headers:

```bash
curl -I http://127.0.0.1:8080/
```

Expected result:
- CORS and custom headers are present.
- Timeout policy is active for request parsing.

## 3. Enable High-Performance Serving

You want async-first serving and benchmarkable throughput.

### Steps

1. Run the benchmark target with high-performance features:

```bash
HTTP_HANDLE_MODE=high-perf \
HTTP_HANDLE_ADDR=127.0.0.1:8090 \
HTTP_HANDLE_ROOT=./target/perf-root \
cargo run --example benchmark_target --features async,high-perf
```

2. In another terminal, run the benchmark matrix:

```bash
bash scripts/perf/benchmark_matrix.sh
```

Expected result:
- Server starts and passes readiness probes.
- Benchmark output shows non-zero `Reqs/sec` and successful `2xx` responses.

## 4. Validate Feature-Specific Behavior

You want confidence that optional modules behave correctly.

Use focused examples:
- Async runtime: `cargo run --example feature_async_runtime --features async`
- HTTP/2 path: `cargo run --example feature_http2_server --features http2`
- Auto-tuning: `cargo run --example feature_runtime_autotune --features autotune`
- Tenant isolation: `cargo run --example feature_tenant_isolation --features multi-tenant`

## 5. Production Validation Checklist

Run this sequence before release:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --examples --all-features
./scripts/score_docs.sh
./scripts/score_docs_api_surface.sh
```

Then publish docs preview:

```bash
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --no-deps
```
