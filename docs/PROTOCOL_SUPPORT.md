# Protocol Support

This document summarizes protocol capabilities in `http-handle`.

## HTTP/2

- Feature: `http2`
- Implementation: `src/http2_server.rs`
- Capabilities:
  - h2c server start and connection handling
  - request mapping to internal request model
  - static file serving path support
  - protocol behavior tests in unit/integration suites

## HTTP/3 (QUIC Profile)

- Feature: `http3-profile`
- Implementation: `src/http3_profile.rs`
- Capabilities:
  - production baseline profile
  - ALPN route resolution (`h3`, `h2`, `http/1.1`)
  - explicit fallback chain generation for graceful downgrade

This keeps protocol negotiation and fallback behavior explicit while HTTP/3
transport integration remains modular.

## Connection Pooling

- Implementation: `ConnectionPool` in `src/server.rs`
- Features:
  - bounded acquisition
  - active count metrics
  - backpressure behavior tests

## Benchmark Coverage

- Benchmark target: `examples/benchmark_target.rs`
- Modes:
  - `sync`
  - `async`
  - `high-perf`
  - `http2`
- CI perf matrix: `.github/workflows/perf-regression.yml`
