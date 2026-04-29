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
  <a href="https://docs.rs/http-handle"><img src="https://img.shields.io/docsrs/http-handle?style=for-the-badge&logo=docs.rs&label=docs.rs" alt="Docs.rs" /></a>
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

HTTP Handle is a lightweight static-file HTTP server library. Start with `Server::new` for quick prototyping, graduate to `ServerBuilder` for policy configuration, and switch to the async `perf_server` modes (`high-perf` / `high-perf-multi-thread`) for production throughput. Single binary, no runtime dependencies beyond libc.

**v0.0.5 highlights:**

- **Static-file fast path**: pre-serialised response cache + sendfile (Linux) hits **180 k req/s** on Linux/arm64 (`high-perf-mt` mode) — see [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md).
- **HTTP/1.1 keep-alive** on both the sync and async server paths.
- **Five start modes**: `start()`, `start_with_thread_pool(n)`, `start_with_pooling(workers, max)`, `start_with_graceful_shutdown(timeout)`, plus the async `start_high_perf` / `start_high_perf_multi_thread`.
- **HTTP/2 (h2c)** with `feature = "http2"`.
- **HTTP/3 ALPN routing + fallback chain** with `feature = "http3-profile"` (QUIC termination is on the v0.2 roadmap; see [`docs/HTTP3_DESIGN.md`](docs/HTTP3_DESIGN.md)).
- **Enterprise primitives**: TLS / mTLS policy, API-key + JWT verifiers, RBAC + ABAC, hot-reload TOML config, multi-tenant isolation, distributed rate limiter, OTLP-shaped telemetry.

---

## Features

| Capability | Default? | Feature flag |
| :--- | :--- | :--- |
| **Static file serving** with auto MIME detection | yes | — |
| **`ServerBuilder`** (CORS, custom headers, timeouts, rate limit, cache TTL) | yes | — |
| **HTTP/1.1 keep-alive** (RFC 7230 §6.3) | yes | — |
| **`ThreadPool` + `ConnectionPool`** for bounded resources | yes | — |
| **Graceful shutdown** with configurable drain timeout | yes | — |
| **Directory traversal protection** + path sanitisation | yes | — |
| **64 MiB cap on buffered response bodies** (OOM guard) | yes | — |
| **Sharded rate limiter** (16-way) + **ETag LRU cache** | yes | — |
| **Precompressed asset negotiation** (br / zstd / gzip) | yes | — |
| **`LanguageDetector`** (built-in + custom regex patterns) | yes | — |
| **Async runtime helpers** + async server (`start_async`) | opt-in | `async` |
| **High-perf async server** (semaphore-bounded, sendfile) | opt-in | `high-perf` |
| **Multi-thread Tokio runtime entry point** | opt-in | `high-perf-multi-thread` |
| **Pre-serialised response cache** | with `high-perf` | `high-perf` |
| **Concurrent batch file reads** | opt-in | `batch` |
| **Pull-based chunked streaming** (`ChunkStream`) | opt-in | `streaming` |
| **Const MIME table + bitset language detection** | opt-in | `optimized` |
| **Structured tracing** (`tracing` + subscriber) | opt-in | `observability` |
| **HTTP/2 server** (h2c) | opt-in | `http2` |
| **HTTP/3 ALPN profile + fallback chain** | opt-in | `http3-profile` |
| **Distributed rate limiter** (pluggable backend) | opt-in | `distributed-rate-limit` |
| **Multi-tenant config + scoped secrets** | opt-in | `multi-tenant` |
| **Host-profile-derived auto-tuning** | opt-in | `autotune` |
| **TLS / mTLS policy primitives** | opt-in | `enterprise` (umbrella for `tls`, `auth`, `config`, `observability`) |
| **API-key + JWT + RBAC/ABAC enforcement** | opt-in | `enterprise` |
| **TOML config + hot-reload watcher** | opt-in | `enterprise` |

`#![deny(unsafe_code)]` at the crate root with three documented exceptions (libc::sendfile, two test-module env-var mutations).

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

**The friction-free way:** `scripts/example.sh` knows the feature
mapping, so you don't need to remember which Cargo flag each demo
needs.

```bash
./scripts/example.sh hello              # core demos: no features
./scripts/example.sh enterprise         # auto-attaches --features enterprise
./scripts/example.sh dhat --release     # extra cargo flags pass through
./scripts/example.sh --list             # print every name
```

`cargo run --example all` builds and drives every demo in sequence.

**Core** (no features required):

| Example | Command | What it shows |
|---|---|---|
| `hello` | `cargo run --example hello` | Minimal `Server::new` |
| `builder` | `cargo run --example builder` | `ServerBuilder` fluent API (CORS, headers, timeouts, validation) |
| `request` | `cargo run --example request` | `Request::from_stream` parse over a real TCP roundtrip |
| `response` | `cargo run --example response` | `Response::send` + `set_connection_header` |
| `errors` | `cargo run --example errors` | `ServerError` constructors |
| `policies` | `cargo run --example policies` | CORS / security headers / timeouts / rate-limit / cache TTL |
| `pool` | `cargo run --example pool` | `ThreadPool` and `ConnectionPool` bounded-resource semantics |
| `shutdown` | `cargo run --example shutdown` | `ShutdownSignal` lifecycle and graceful drain |
| `keepalive` | `cargo run --example keepalive` | HTTP/1.1 keep-alive over one TCP connection (5 GETs) |
| `language` | `cargo run --example language` | `LanguageDetector` built-in + custom regex patterns |

**Per Cargo feature flag** (one example per flag — copy-paste ready):

| Example | Command | What it shows |
|---|---|---|
| `async` | `cargo run --features async --example async` | `run_blocking` + `start_async` |
| `batch` | `cargo run --features batch --example batch` | Concurrent file reads with parallelism cap |
| `streaming` | `cargo run --features streaming --example streaming` | `ChunkStream` chunked file iteration |
| `optimized` | `cargo run --features optimized --example optimized` | Const MIME table + bitset language detection |
| `observability` | `cargo run --features observability --example observability` | Structured tracing via `tracing-subscriber` |
| `http2` | `cargo run --features http2 --example http2` | h2c server + framed body roundtrip |
| `http3` | `cargo run --features http3-profile --example http3` | ALPN routing + fallback chain |
| `perf` | `cargo run --features high-perf --example perf` | `start_high_perf` with `PerfLimits` |
| `multi` | `cargo run --features high-perf-multi-thread --example multi` | `start_high_perf_multi_thread` |
| `autotune` | `cargo run --features autotune --example autotune` | Host-profile-derived `PerfLimits` |
| `ratelimit` | `cargo run --features distributed-rate-limit --example ratelimit` | Distributed limiter + in-memory backend |
| `tenant` | `cargo run --features multi-tenant --example tenant` | Per-tenant config + scoped secrets |
| `tls` | `cargo run --features enterprise --example tls` | TLS / mTLS policy primitives |
| `auth` | `cargo run --features enterprise --example auth` | API-key + JWT verifiers |
| `config` | `cargo run --features enterprise --example config` | TOML config + hot-reload watcher |
| `enterprise` | `cargo run --features enterprise --example enterprise` | RBAC adapter + per-request enforcement |

**Tooling / runners**:

| Example | Command | What it does |
|---|---|---|
| `full` | `cargo run --example full` (or `--all-features` for full coverage) | Unified runner across every enabled feature |
| `all` | `cargo run --example all` | Spawns every other example via `cargo run --example` |
| `bench` | `scripts/load_test.sh <mode>` | Bombardier target — sync / async / high-perf / high-perf-mt / http2 |
| `dhat` | `cargo run --release --features high-perf --example dhat` | Heap-profile harness writing `dhat-heap.json` |

---

## FAQ

Short answers to the questions that come up most often. The long-form
matrix lives in [`docs/FAQ.md`](docs/FAQ.md).

**Which `start*` should I use?** Default to `Server::start()` for
prototyping. For production: `start_with_pooling(workers, max_conns)`
on the sync path, or `start_high_perf_multi_thread(server, limits, None)`
on the async path. The async multi-thread mode is the throughput leader
on every Linux row in [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md).

**When does sync win, and when does async win?** Sync's
thread-per-connection model is competitive on memory-rich hosts with
zero per-request I/O wait. Async (`high-perf` / `high-perf-mt`) wins on
memory-constrained hosts, mixed-I/O workloads (DB / upstream awaits),
and any deployment where the OS thread cap matters.

**Are the perf numbers realistic for my workload?** They're upper
bounds for a small static body over loopback. The Linux row (180 k
req/s on `high-perf-mt`) is more representative for production than
the macOS row because it tests the real `epoll` networking path.
Reproduce with `scripts/load_test.sh` (host) or `scripts/linux_bench.sh`
(container).

**Can I run this over HTTPS?** Not in-process today —
`enterprise::TlsPolicy` is configuration-only. Terminate TLS at a
reverse proxy (nginx, Caddy, ALB) and proxy plaintext HTTP/1.1 or h2c
to `http-handle` over loopback. In-process `rustls` termination is on
the v0.1 roadmap.

**What about HTTP/3?** ALPN routing + fallback chain ship as
`feature = "http3-profile"`. QUIC termination + h3 frames are deferred
to v0.2 (design proposal in [`docs/HTTP3_DESIGN.md`](docs/HTTP3_DESIGN.md)).

**Can I plug in my own request handler?** Not yet — the crate is a
static-file server with configurable policies. A request-handler trait
is on the v0.1 roadmap. Today, run `http-handle` for `/static/*` and a
dynamic framework (`axum`, `actix-web`) for the rest.

**`error: target 'enterprise' requires the features: 'enterprise'` —
how do I run feature-gated examples?** Use the wrapper:
`./scripts/example.sh <name>`. It auto-resolves the right `--features`
flag. Or copy the literal command from the `## Examples` table above.

**How does the response cache work?** v0.0.5 added a pre-serialised
`(head_prefix, body)` cache on the `high-perf` static-file fast path,
keyed by `(canonical_path, mtime, file_len)`, capped at 256 entries
~16 MiB total. Hits skip the syscall + format work; the per-request
cost on a hit is one `Connection:` header format and one `write_all`.
Above-threshold files (>= `sendfile_threshold_bytes`) skip the cache
entirely so the OOM guard remains intact.

For deployment patterns (nginx, Kubernetes), security model (rate
limiter, traversal protection, body cap), tuning knobs, and 20+ more
answers, see [`docs/FAQ.md`](docs/FAQ.md).

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