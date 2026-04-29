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
  - QUIC runtime tuning presets (`Conservative`, `Balanced`, `Aggressive`)
  - ALPN route resolution (`h3`, `h2`, `http/1.1`)
  - client-offered ALPN selection with server-order policy
  - explicit fallback chain generation for graceful downgrade
  - decision tree with fallback reasons and telemetry line output

### HTTP/3 Conformance Tests

- Integration tests: `tests/http3_profile_conformance.rs`
- Covered paths:
  - ALPN matrix route stability
  - h3-handshake failure downgrade behavior
  - server-preferred ALPN policy enforcement

This keeps protocol negotiation and fallback behavior explicit while HTTP/3
transport integration remains modular.

## Connection Pooling

- Implementation: `ConnectionPool` in `src/server.rs`
- Features:
  - bounded acquisition
  - active count metrics
  - backpressure behavior tests

## Benchmark Coverage

- Benchmark target: `examples/bench.rs`
- Modes (selected via `HTTP_HANDLE_MODE`):
  - `sync`
  - `async`
  - `high-perf`
  - `high-perf-mt`
  - `http2`
- CI perf matrix: `.github/workflows/perf-regression.yml`
