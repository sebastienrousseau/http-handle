# Examples Coverage Matrix

This document maps `http-handle` capabilities to runnable examples so
functional coverage stays explicit across code, docs, and CI. Every
listed name corresponds to `examples/<name>.rs` and is registered as a
`[[example]]` target in `Cargo.toml`.

## Run any example

```shell
cargo run --example <name> [--features "<flag>"]
```

The `support.rs` helper module is shared via
`#[path = "support.rs"] mod support;` and is not invoked directly.
`cargo run --example all` exercises every demo in sequence.

## Core (no Cargo features)

| Capability | Example |
|---|---|
| Minimal `Server::new` | `hello` |
| `ServerBuilder` fluent configuration | `builder` |
| `Request::from_stream` over a real TCP connection | `request` |
| `Response::send` and `set_connection_header` | `response` |
| `ServerError` constructors | `errors` |
| CORS / headers / timeouts / rate-limit / cache TTL | `policies` |
| `ThreadPool` and `ConnectionPool` resource caps | `pool` |
| `ShutdownSignal` lifecycle and graceful drain | `shutdown` |
| HTTP/1.1 keep-alive over a single TCP connection | `keepalive` |
| `LanguageDetector` built-in + custom regex patterns | `language` |

## Per Cargo feature

| Capability | Feature | Example |
|---|---|---|
| Async runtime helper + `start_async` | `async` | `async` |
| Concurrent batch reads | `batch` | `batch` |
| `ChunkStream` chunked file iteration | `streaming` | `streaming` |
| Const MIME table + bitset language detection | `optimized` | `optimized` |
| Structured tracing via `tracing-subscriber` | `observability` | `observability` |
| HTTP/2 (h2c) server roundtrip | `http2` | `http2` |
| HTTP/3 ALPN routing + fallback chain | `http3-profile` | `http3` |
| `start_high_perf` with `PerfLimits` | `high-perf` | `perf` |
| `start_high_perf_multi_thread` | `high-perf-multi-thread` | `multi` |
| Host-profile auto-tuning | `autotune` | `autotune` |
| Distributed rate limiter + in-memory backend | `distributed-rate-limit` | `ratelimit` |
| Per-tenant config + scoped secrets | `multi-tenant` | `tenant` |
| TLS / mTLS policy primitives | `enterprise` | `tls` |
| API-key + JWT verifiers | `enterprise` | `auth` |
| TOML config + hot-reload watcher | `enterprise` | `config` |
| RBAC adapter + `enforce_http_request_authorization` | `enterprise` | `enterprise` |

## Tooling / runners

| Capability | Example |
|---|---|
| Unified runner across enabled features | `full` |
| Sequential runner that drives every other example | `all` |
| Bombardier benchmark target (env-driven mode) | `bench` |
| dhat heap-profile harness | `dhat` |

## Validation commands

```shell
# compile every example with all optional features enabled
cargo check --all-features --examples

# run every example sequentially
cargo run --example all

# run all tests and doctests
cargo test --all-features
```
