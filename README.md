<p align="center">
  <img src="https://cloudcdn.pro/http-handle/v1/logos/http-handle.svg" alt="HTTP Handle logo" width="128" />
</p>

<h1 align="center">HTTP Handle</h1>

<p align="center">
  <strong>A fast and lightweight Rust library for handling HTTP requests and responses.</strong>
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/http-handle/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/http-handle/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/http-handle"><img src="https://img.shields.io/crates/v/http-handle.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/http-handle"><img src="https://img.shields.io/badge/docs.rs-http-handle-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://codecov.io/gh/sebastienrousseau/http-handle"><img src="https://img.shields.io/codecov/c/github/sebastienrousseau/http-handle?style=for-the-badge&logo=codecov" alt="Coverage" /></a>
  <a href="https://lib.rs/crates/http-handle"><img src="https://img.shields.io/badge/lib.rs-v0.0.5-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
</p>

---

## Install

```bash
cargo add http-handle
```

Or add to `Cargo.toml`:

```toml
[dependencies]
http-handle = "0.0.5"
```

You need [Rust](https://rustup.rs/) 1.88.0 or later. Works on macOS, Linux, and Windows.

---

## Overview

HTTP Handle is a lightweight HTTP server library for serving static websites. Start with `Server::new` for quick prototyping, then graduate to `ServerBuilder` for production control.

- **Static file serving** with automatic MIME detection
- **Fluent builder API** for CORS, headers, and timeouts
- **Graceful shutdown** with configurable drain timeout
- **Directory traversal protection** built in

---

## Features

| | |
| :--- | :--- |
| **Static file serving** | Route requests to a document root with MIME detection |
| **ServerBuilder** | Fluent API for CORS, headers, and timeouts |
| **Graceful shutdown** | Signal-aware shutdown with configurable drain |
| **Security** | Directory traversal protection and path sanitisation |
| **Precompressed assets** | Content negotiation for br, gzip, and zstd |
| **TLS / mTLS** | TLS and mutual-TLS configuration primitives |

---

## Usage

```rust,no_run
use http_handle::Server;

fn main() -> std::io::Result<()> {
    let server = Server::new("127.0.0.1:8080", "./public");
    server.start()
}
```

For a complete capability-to-example matrix (basic server, builder pattern,
graceful shutdown, feature-gated modules like async / HTTP/2 / streaming /
enterprise auth), see [docs/EXAMPLES.md](docs/EXAMPLES.md).

---

## Examples

30 one-word examples cover every public API and feature flag. Each is
registered as a `[[example]]` target in `Cargo.toml` and follows a
shared layout (animated spinner + checkmark output via
`examples/support.rs`).

```bash
cargo run --example <name> [--features "<flag>"]
cargo run --example all                  # run every demo in sequence
```

**Core** (no features required):

| Example | What it shows |
|---|---|
| `hello` | Minimal `Server::new` |
| `builder` | `ServerBuilder` fluent API (CORS, headers, timeouts, validation) |
| `request` | `Request::from_stream` parse over a real TCP roundtrip |
| `response` | `Response::send` + `set_connection_header` |
| `errors` | `ServerError` constructors |
| `policies` | CORS / security headers / timeouts / rate-limit / cache TTL |
| `pool` | `ThreadPool` and `ConnectionPool` bounded-resource semantics |
| `shutdown` | `ShutdownSignal` lifecycle and graceful drain |
| `keepalive` | HTTP/1.1 keep-alive over one TCP connection (5 GETs) |
| `language` | `LanguageDetector` built-in + custom regex patterns |

**Per Cargo feature flag** (one example per flag):

| Example | Feature | What it shows |
|---|---|---|
| `async` | `async` | `run_blocking` + `start_async` |
| `batch` | `batch` | Concurrent file reads with parallelism cap |
| `streaming` | `streaming` | `ChunkStream` chunked file iteration |
| `optimized` | `optimized` | Const MIME table + bitset language detection |
| `observability` | `observability` | Structured tracing via `tracing-subscriber` |
| `http2` | `http2` | h2c server + framed body roundtrip |
| `http3` | `http3-profile` | ALPN routing + fallback chain |
| `perf` | `high-perf` | `start_high_perf` with `PerfLimits` |
| `multi` | `high-perf-multi-thread` | `start_high_perf_multi_thread` |
| `autotune` | `autotune` | Host-profile-derived `PerfLimits` |
| `ratelimit` | `distributed-rate-limit` | Distributed limiter + in-memory backend |
| `tenant` | `multi-tenant` | Per-tenant config + scoped secrets |
| `tls` | `enterprise` | TLS / mTLS policy primitives |
| `auth` | `enterprise` | API-key + JWT verifiers |
| `config` | `enterprise` | TOML config + hot-reload watcher |
| `enterprise` | `enterprise` | RBAC adapter + per-request enforcement |

**Tooling / runners**:

| Example | What it does |
|---|---|
| `full` | Unified runner across every enabled feature |
| `all` | Spawns every other example via `cargo run --example` |
| `bench` | Bombardier target — driven by `scripts/load_test.sh` |
| `dhat` | Heap-profile harness writing `dhat-heap.json` |

---

## Migrating from 0.0.4 → 0.0.5

`Request::headers` changed from `HashMap<String, String>` to `Vec<(String, String)>` for lower per-request allocation pressure and faster lookup at typical header counts. The `Request::header(name)` accessor is unchanged; only direct construction and iteration are affected.

```rust,ignore
// before (0.0.4)
let mut headers = std::collections::HashMap::new();
headers.insert("content-type".to_string(), "text/plain".to_string());
let request = Request {
    method: "GET".into(),
    path: "/".into(),
    version: "HTTP/1.1".into(),
    headers,
};

// after (0.0.5)
let request = Request {
    method: "GET".into(),
    path: "/".into(),
    version: "HTTP/1.1".into(),
    headers: vec![("content-type".into(), "text/plain".into())],
};
```

See [CHANGELOG.md](CHANGELOG.md#005---2026-04-26) for the full breaking-change list.

---

## Development

```bash
cargo build        # Build the project
cargo test         # Run all tests
cargo clippy       # Lint with Clippy
cargo fmt          # Format with rustfmt
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, signed commits, and PR guidelines.

---

**THE ARCHITECT** ᛫ [Sebastien Rousseau](https://sebastienrousseau.com)
**THE ENGINE** ᛞ [EUXIS](https://euxis.co) ᛫ Enterprise Unified Execution Intelligence System

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT), at your option.

<p align="right"><a href="#http-handle">Back to Top</a></p>