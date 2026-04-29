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
http-handle = "0.0.5"
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
        .enable_cors()
        .custom_header("X-Content-Type-Options", "nosniff")
        .build()
        .expect("server build");

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
cargo run --release --example bench --features 'async,high-perf,high-perf-multi-thread,http2'
```

2. In another terminal, run the bombardier driver:

```bash
./scripts/load_test.sh high-perf 30 256
# Modes: sync, async, high-perf, high-perf-mt, http2
```

Expected result:
- Server starts and passes readiness probes.
- Benchmark output shows non-zero `Reqs/sec` and successful `2xx` responses.

## 4. Validate Feature-Specific Behavior

You want confidence that optional modules behave correctly. Each
feature has a one-word example registered in `Cargo.toml`. The
friction-free way is the wrapper that auto-resolves the feature flag:

```bash
./scripts/example.sh async
./scripts/example.sh http2
./scripts/example.sh autotune
./scripts/example.sh tenant
./scripts/example.sh enterprise
./scripts/example.sh --list      # see every example name
```

Or via raw cargo:

```bash
cargo run --features async        --example async
cargo run --features http2        --example http2
cargo run --features autotune     --example autotune
cargo run --features multi-tenant --example tenant
cargo run --features enterprise   --example enterprise
```

The full capability → example matrix lives in [docs/EXAMPLES.md](EXAMPLES.md).

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

## 6. Error Recovery and Deprecation Readiness

Before release, verify operational docs are aligned with runtime behavior:

1. Review common failure scenarios and recovery actions:
   - `docs/ERRORS_AND_RECOVERY.md`
2. Confirm deprecation and migration guidance is up to date:
   - `docs/DEPRECATION_POLICY.md`
3. Ensure release notes and migration details are synchronized:
   - `CHANGELOG.md`
   - `docs/RELEASE_TRANSITION_v0.0.3.md`
