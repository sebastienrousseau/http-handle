# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project follows Semantic Versioning.

## [0.0.5] - 2026-04-26

### ⚠️ Breaking

- **`Request::headers` is now `Vec<(String, String)>` (was `HashMap<String, String>`).** Linear scan beats hashing at typical header counts (≤32) and removes the per-request hashmap allocation. Callers that built requests by hand should migrate from `HashMap::new()` + `.insert(name, value)` to `vec![(name, value)]` or `Vec::with_capacity(N)` + `.push((name, value))`. The `Request::header(name)` accessor is unchanged.
- **`Request::headers()` now returns `&[(String, String)]` (was `&HashMap<String, String>`).** Iteration via `.iter()` continues to work; calls to map-specific methods (`get`, `contains_key`) need to switch to a linear-scan equivalent or use `Request::header`.
- **`#![forbid(unsafe_code)]` at crate root replaced with `#![deny(unsafe_code)]` plus targeted `#[allow(unsafe_code)]` on three authorized sites** (`libc::sendfile` in `perf_server::try_sendfile_unix`, env-var mutations in `tenant_isolation` and `runtime_autotune` test modules). The crate-wide guarantee is preserved; downstream code that imported the lint level via reflection won't see a difference.
- **`Server` gained a `canonical_document_root: PathBuf` field** (cached at build time so the request hot path stops issuing two `fs::canonicalize` syscalls per request). The field is `#[serde(skip)]` so the wire shape of `Server` serde output is unchanged, but anyone that constructed `Server` via struct-literal syntax outside the `ServerBuilder` will need `Default::default()` on the new field.

### Added

- **HTTP/1.1 keep-alive** (RFC 7230 §6.3) on the synchronous server path. Up to 100 requests per connection, 5 s idle timeout between requests. HTTP/1.0 defaults to close; explicit `Connection: keep-alive` / `Connection: close` overrides the version default.
- **Multi-thread async runtime** — new `high-perf-multi-thread` feature flag and `perf_server::start_high_perf_multi_thread(server, limits, worker_threads)` entry point. Builds a Tokio multi-thread runtime internally so callers don't need to add `rt-multi-thread` to their tokio features. `worker_threads = None` defaults to logical CPU count; `Some(n)` pins worker count for reproducible benchmarking or container CPU caps. `benchmark_target` example exposes the new mode as `HTTP_HANDLE_MODE=high-perf-mt` with optional `HTTP_HANDLE_WORKERS=N`. Honest comparison numbers and tradeoff guidance in `docs/PERFORMANCE.md`: on a CPU-bound static-file workload the sync `Server::start` thread-per-connection model still wins, while `high-perf-mt` is the right primitive for mixed I/O workloads (backend awaits, slow downstreams).
- **Sharded rate limiter** — `[Mutex<HashMap<IpAddr, Vec<Instant>>>; 16]` keyed by `DefaultHasher` of the client IP. Cuts effective contention 16× compared to the previous global mutex when distinct clients hit the limiter concurrently.
- **ETag LRU cache** — `OnceLock<RwLock<HashMap<(u64, u64), Arc<str>>>>` keyed by `(file_len, mtime_secs)` with 256-entry cap. Cache hits return `Arc::clone` (refcount bump) instead of a fresh `format!` allocation.
- **HTTP/3 design proposal** — `docs/HTTP3_DESIGN.md` captures the QUIC dependency choice (quinn + h3 + h3-quinn over s2n-quic), module layout, threading model, and milestone breakdown for the v0.2 spike.
- **`dhat_alloc_profile` example** — `cargo run --release --example dhat_alloc_profile --features high-perf` writes `dhat-heap.json` for offline allocation analysis.
- **Bench targets** — 9 new criterion benches across sync / perf_server / h2 / concurrent (8/32/64 fan-outs) plus rate-limit-on, shutdown-aware, h2 single-stream, and `Response::send` micro.
- **`memchr` direct dependency** for SIMD `:` separator search in header-line parsing (NEON on Apple Silicon, AVX2 on x86_64). Already transitively present, no new compile cost.
- **`crossbeam-channel` direct dependency** for the lock-free `ThreadPool` worker queue.

### Performance

- Sync server hot path **621 µs → 438 µs p50** on small-body roundtrip (1.42×). Driven by buffered `BufWriter` response writes (6+ syscalls → 1), cached canonical document root (saves `fs::canonicalize` per request), and `TCP_NODELAY` on every accept site.
- Shutdown-aware path **~103 ms → 895 µs p50** on single-client roundtrip. The 100 ms `WouldBlock` sleep-poll between non-blocking accept calls replaced with an adaptive 100 µs–5 ms backoff that resets on accept.
- Async `perf_server` static-file fast path now uses `std::fs::canonicalize` / `metadata` / `read` inside the async handler instead of `tokio::fs::*`, eliminating 5 `spawn_blocking` round-trips per request on the small-file (sub-`sendfile_threshold_bytes`) hot path. For local-disk files those syscalls return in microseconds, so the blocking-pool hop cost more than the work. Bombardier (256-conn keep-alive, 30 s, Apple Silicon): `start_high_perf` **9,583 → 14,481 req/s (+51%)**, p99 51 ms → 20 ms; `start_high_perf_multi_thread` **8,914 → 22,510 req/s (+153%)**, p99 77 ms → 23 ms. Above-threshold path unchanged (still uses `tokio::fs::File::open` + `tokio::io::copy` or `sendfile`).
- **Response cache on the high-perf static-file fast path.** Pre-serialised `(head_prefix, body)` keyed by `(canonical_path, mtime_secs, file_len)` with `RESPONSE_CACHE_MAX = 256` entries and cap-based eviction that mirrors the existing ETag LRU. Body-size-gated by `sendfile_threshold_bytes` so worst-case memory is 16 MiB (256 × 64 KiB). Hit path: one `format!` for the per-request `Connection` header, one `extend_from_slice` of cached prefix + cached body into a single buffer, one `write_all` syscall. Bombardier on macOS: `start_high_perf` **14,481 → 27,971 req/s (+93%)**, `start_high_perf_multi_thread` **22,510 → 32,181 req/s (+43%, now beats sync at 29,815)**. Linux/arm64 in container hits **180,765 req/s** on `high-perf-mt`. New `scripts/linux_bench.sh` reproduces the Linux numbers in a podman/Docker container. Above-threshold path skips the cache entirely so the OOM guard remains intact.
- Async `perf_server` path no longer issues blocking syscalls inside async fn bodies for the *above-threshold* file copy path — `tokio::fs::*` and `spawn_blocking` retained for files past `sendfile_threshold_bytes`. Earlier per-request latency under 8-way concurrency: **456 µs/req** (vs 579 µs single-client; per-request *drops* under load, confirming the reactor no longer stalls).
- `ThreadPool` worker queue swapped from `Arc<Mutex<Receiver>>` to `crossbeam-channel` MPMC. Removes the single-mutex serialisation point that capped scaling at 3–4 cores under sustained load.
- Header lookup (`Request::header`) now linear-scans a `Vec<(String, String)>` instead of hashing into a `HashMap<String, String>`. Wins on parse and lookup at typical header counts.

### Security

- **64 MiB cap on buffered response bodies** in `serve_file_response`. A pre-flight `fs::metadata` check rejects oversize files with `ServerError::Custom` mapped to 503 — closes the OOM vector where a 1 GB file load would drive RSS to N × file_size on N concurrent requests. True streaming (write-as-read into the wire) is parked for v0.1 pending the `ResponseBody` enum API redesign.
- `cargo deny` policy expanded to four-category coverage (advisories, bans, licenses, sources) with explicit allow-list including the project's `AGPL-3.0-only` self-license.
- `Cargo.lock` committed for reproducible CI and `cargo audit` enforcement.

### Changed

- Test coverage **95.82% → 99.10%** crate-wide. Lib test count **150 → 212**. Every targeted module now ≥ 98% line coverage.
- 30+ bespoke GitHub Actions workflows retired in favor of three reusable-workflow calls into `sebastienrousseau/pipelines` (rust-ci, security, docs). Dependabot moved to weekly Monday 09:00 UTC schedule with grouped minor/patch updates.
- `release.opt-level` switched from `"s"` (size) to `3` (speed) — server is latency / throughput-sensitive; binary size is not the constraint for a library consumed as a dependency.
- SPDX copyright header updated to `Copyright (c) 2023 - 2026 HTTP Handle` across 82 files.
- Rustdoc lints tightened: `rustdoc::broken_intra_doc_links`, `rustdoc::bare_urls`, `rustdoc::invalid_html_tags` all denied.

### Fixed

- Linux + `high-perf` builds previously failed under `forbid(unsafe_code)` because `libc::sendfile` requires an `unsafe` block; the `deny + allow` switch unblocks 11 previously-uncompilable tests in `tenant_isolation` and `runtime_autotune` (test count jumped 190 → 201 once those re-entered the build).
- Bench harness no longer uses hard-coded port 8082 (collision under concurrent runs) or leaks server threads. Probe-based port discovery + full-body reads on the client side eliminate the `Connection reset by peer` stderr noise.

## [0.0.4] - 2026-03-09

### Changed
- Release commit aligned to current `main` after PR #81 merge.
- Crate publication flow finalized through tag-driven `publish-crates` workflow.

## [0.0.3] - 2026-02-22

### Added
- Async-first high-performance serving path foundations with adaptive backpressure controls.
- Precompressed static asset negotiation for `.br`, `.zst`, and `.gz` variants.
- Enterprise-oriented policy foundations: profiles, auth primitives, audit events, and hot-reload hooks.
- Portability conformance tests and cross-platform CI/release matrix expansion.
- HTTP/3 profile hardening with fallback policy and protocol conformance coverage.
- Expanded examples and module-level test coverage across new functionality.

### Changed
- CI/security workflows hardened with pinned actions, strict policy gates, and improved supply-chain controls.
- Documentation structure and API narrative expanded for production-readiness and discoverability.
- Coverage enforcement moved to strict `>=95%` line threshold gates.

### Fixed
- Multiple CI reproducibility and workflow consistency issues across feature and release pipelines.
- Dependabot-driven workflow drift resolved directly on `feat/v0.0.3`.

## [0.0.2] - 2026-02-04

### Added
- Initial baseline features and project scaffolding for static HTTP serving and core request/response handling.
