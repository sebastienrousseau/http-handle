# HTTP/3 Server Design

**Status:** design / proposal — no executable HTTP/3 server is shipped in v0.0.5. The crate currently exposes `http3_profile` (ALPN routing and fallback policy) but does **not** terminate QUIC or speak h3 frames. This document captures the implementation plan and the decisions that have to be made before code lands.

## Why this is a design doc, not stubs

Per the project's policy on partial implementations, we don't ship `http3_server::start_http3` returning `unimplemented!()`. Either the path works end-to-end (QUIC handshake, h3 frames, body serving, graceful shutdown) or it doesn't exist. Stubs invite callers to depend on signatures that will change when the real impl lands.

## Scope

A minimal v0.2 HTTP/3 server has to:

1. Bind a UDP socket, accept QUIC connections, terminate TLS-1.3 with ALPN negotiation.
2. Drive `h3::server::Connection` per accepted QUIC connection.
3. For each h3 stream, parse the request, route through the existing `build_response_for_request_with_metrics` to share behavior with HTTP/1 and HTTP/2.
4. Serialize the response back through h3 frames.
5. Respect `Http3ProductionProfile::resolve_route` for fallback decisions when a client offers ALPN tokens we don't speak.
6. Integrate with `ShutdownSignal` for graceful drain.

## Dependency choice: `quinn` vs `s2n-quic`

| | `quinn` | `s2n-quic` |
|---|---|---|
| QUIC stack maturity | de-facto Rust default; widely deployed | newer; AWS-backed |
| TLS backend | `rustls` (pure Rust) | `s2n-tls` (C, FIPS-validated) |
| h3 integration | `h3` + `h3-quinn` (paired) | `h3` + `h3-s2n-quic` (less mature) |
| Async runtime | tokio | tokio |
| MSRV | rolling | rolling |
| License | Apache-2.0 OR MIT | Apache-2.0 |

**Recommendation:** `quinn` + `h3` + `h3-quinn`. Pure-Rust TLS-1.3 via `rustls` aligns with the existing crate posture (no C deps in the hot path), `h3-quinn` is the canonical glue, and the project is the most actively maintained.

Cost to declare:
```
quinn = "0.11"           # QUIC transport
h3 = "0.0"               # HTTP/3 framing on top of QUIC
h3-quinn = "0.0"         # quinn ↔ h3 bridge
rustls = "0.23"          # TLS 1.3
rustls-pemfile = "2"     # PEM cert/key parsing
tokio-rustls = "0.26"    # tokio integration
```
This is a substantial addition; the `http3` feature flag must remain off by default.

## Module layout

```
src/
  http3_profile.rs   ← ALPN/fallback policy (already shipped in v0.0.5)
  http3_server.rs    ← NEW: start_http3(server, tls_config) accept loop
```

Public surface (planned):

```rust
#[cfg(feature = "http3")]
pub async fn start_http3(
    server: Server,
    tls: Arc<rustls::ServerConfig>,
    bind_addr: SocketAddr,
) -> Result<(), ServerError>;
```

Identical shape to `start_http2`/`start_high_perf` so callers can swap implementations behind a config flag.

## Threading model

QUIC is connection-oriented over UDP; tokio's `current_thread` runtime can drive a single accept loop. Each accepted `quinn::Connecting` is `tokio::spawn`'d to run h3 handshake + stream multiplexing concurrently. h3 streams within a connection share the connection driver task; per-stream work runs on the same task with cooperative `await` points.

Same pattern `start_http2` uses today — drop in.

## TLS configuration

The crate already has `enterprise::TlsPolicy` with cert-chain / private-key paths. Reuse it: `start_http3` accepts a `&Server` and reads `server.canonical_document_root` plus an `Arc<rustls::ServerConfig>` that the caller built from the policy. ALPN negotiation populates `["h3", "h2", "http/1.1"]` per `Http3ProductionProfile::alpn_order`.

Fallback when the client doesn't offer `h3` is **not** handled inside `start_http3` itself — the deployment is expected to bind both the QUIC listener (this fn) and a separate TCP listener for HTTP/2 / HTTP/1.1, and `Http3ProductionProfile` advertises that capability via Alt-Svc.

## Milestones

1. **M1 — listener stub.** `start_http3` binds the UDP socket, accepts a QUIC handshake, drops the connection. Smoke test: `quinn` client can complete a handshake. (~1 day)
2. **M2 — h3 echo.** Drive `h3::server::Connection`, accept the first stream, decode the request, echo back a fixed 200 OK with `b"hello-h3"`. (~1 day)
3. **M3 — wire to existing pipeline.** Replace the echo with `build_response_for_request_with_metrics`. Static-file serving, ETag, range — all reuse the HTTP/1 logic. (~1 day)
4. **M4 — graceful shutdown.** Bind to `ShutdownSignal`. On signal, stop accepting new connections, drain in-flight streams, close QUIC connections with `0x100` (`H3_NO_ERROR`). (~0.5 day)
5. **M5 — bench harness.** `benches/http3_benchmark.rs` analogous to `http2_benchmark.rs`. (~0.5 day)
6. **M6 — production hardening.** 0-RTT replay protection, connection migration, congestion control tuning per `QuicTuning`. (~2 days)

Total: ~6 dev-days for an MVP that's not embarrassing.

## Why this isn't in v0.0.5

- The QUIC dependency footprint (~20 transitive crates between `quinn`, `h3`, `rustls`) is too large to vendor without a feature gate, and the gate would default off.
- `rustls` ships its own crypto provider (`ring` or `aws-lc-rs`) which interacts with OS distribution rules. The licensing/provider matrix needs a separate decision.
- 0.0.x releases are pre-1.0 by convention but adding h3 sets stability expectations we're not ready to commit to.

## Open questions

- Should `start_http3` accept a `tokio::net::UdpSocket` (for callers that want to pre-configure socket options) or do the bind itself?
- Should we expose `Alt-Svc` header injection from the HTTP/1 path so clients hitting the TCP listener learn about the h3 endpoint? (Yes, but it's a separate commit.)
- What's the per-connection idle-timeout default? `Http3ProductionProfile::quic_tuning()::idle_timeout_ms` already returns a value; that flows through here.

---

This doc supersedes the placeholder `[T4] HTTP/3 server` audit item. The work isn't done; the path to doing it is documented and the decisions are made. Re-open as v0.2 implementation work when the QUIC dep vetting is approved.
