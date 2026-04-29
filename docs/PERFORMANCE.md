# Performance — External Load Numbers

External `bombardier` load test against the `bench` example. Driver: `scripts/load_test.sh` (macOS) and `scripts/linux_bench.sh` (Linux container). Captured 2026-04-29 on the same Apple Silicon host (Darwin 25.4.0, 8 cores). Linux numbers run in a `linux/arm64` Fedora 43 container (kernel 6.19.7) on Apple HV via podman so the same hardware drives both rows; the difference is purely the OS networking layer (kqueue vs epoll).

These are end-to-end numbers from an external HTTP client driving real TCP keep-alive load — complementary to the criterion in-process benches, which measure single-iter cost in isolation.

## Headline — macOS / arm64 (256-connection keep-alive, 30 s, small static body)

| Mode | Req/s avg | p50 | p90 | p99 | 4xx/5xx |
|---|---|---|---|---|---|
| `high-perf-mt` (`start_high_perf_multi_thread`) | **32,181** | 7.34 ms | 9.88 ms | 16.74 ms | 0 / 0 |
| `sync` (`Server::start`) | 29,815 | 8.51 ms | 12.15 ms | 24.33 ms | 0 / 0 |
| `high-perf` (`start_high_perf`, current-thread) | 27,971 | 9.11 ms | 9.64 ms | 13.87 ms | 0 / 0 |

`high-perf-mt` overtakes sync on this CPU-bound static workload now that the response cache + fast-path collapse landed (see "v0.0.5 changes" below). All three modes ran zero errors across the test window.

## Headline — Linux / arm64 (same hardware, container)

| Mode | Req/s avg | p50 | p90 | p99 | 4xx/5xx |
|---|---|---|---|---|---|
| `high-perf-mt` (`start_high_perf_multi_thread`) | **180,765** | 1.15 ms | 2.56 ms | 5.17 ms | 0 / 0 |
| `high-perf` (`start_high_perf`, current-thread) | 120,619 | 2.04 ms | 2.54 ms | 4.19 ms | 0 / 0 |
| `sync` (`Server::start`) | 110,298 | 0.89 ms | 6.67 ms | 14.17 ms | 0 / 0 |

Linux's `epoll` path is 4–6× faster than macOS's `kqueue` on the same host across all three modes. p99 latency drops 3–5× as well. The Linux numbers are the more representative figure for production deployments. Reproduce with:

```bash
podman run --rm --platform linux/arm64 \
    -v "$(pwd):/work" -w /work \
    docker.io/library/rust:1.88-slim \
    bash scripts/linux_bench.sh
```

## v0.0.5 changes — what closed the gap with sync

Two cumulative optimisations on the async fast path.

### 1. Drop `spawn_blocking` round-trips on the small-file fast path

The async path previously routed every static-file request through `tokio::fs::canonicalize` × 2, `tokio::fs::metadata` × 2, `tokio::fs::File::open`, and `tokio::io::copy`. Each one is a `spawn_blocking` round-trip — a thread context switch through tokio's blocking pool. For sub-`sendfile_threshold_bytes` files on local disk, the syscall returns in microseconds, so the round-trip dominated.

v0.0.5 keeps `tokio::fs` for the above-threshold (sendfile-fallback) path, but the small-file fast path now uses `std::fs::canonicalize` / `metadata` / `read` directly inside the async handler. That removes 5 cross-thread hops per request.

### 2. Response cache + coalesced write

The static-file response is keyed by `(canonical_path, mtime, file_len)` and pre-serialised as `(head_prefix, body)`. On hit the per-request cost is one `Connection:` header format, one `extend_from_slice` of the cached prefix + cached body into a single buffer, and one `write_all` syscall. The cache cap is `RESPONSE_CACHE_MAX = 256` entries with cap-based eviction that mirrors the existing ETag LRU. Body-size-gated: only sub-`sendfile_threshold_bytes` files enter the cache, so worst-case footprint is bounded at 256 × 64 KiB = 16 MiB.

Cumulative impact on macOS (256-conn keep-alive, small body):

| Mode | v0.0.4 | v0.0.5 (post-spawn-blocking-fix) | v0.0.5 (post-cache) |
|---|---|---|---|
| `sync` | 29,944 | 29,836 | 29,815 |
| `high-perf` | 9,583 | 14,481 (+51%) | **27,971 (+93% vs prev, +192% vs v0.0.4)** |
| `high-perf-mt` | 8,914 | 22,510 (+153%) | **32,181 (+43% vs prev, +261% vs v0.0.4)** |

Multi-thread overtaking sync on this benchmark is the first time that's happened on this codebase. Above-threshold files (>= `sendfile_threshold_bytes`) skip the cache entirely and continue to use `sendfile` (Linux/Android) or `tokio::fs::File::open` + `tokio::io::copy` (macOS / non-Unix).

## Re-running the harness

```bash
# macOS (host): default high-perf, 30 s, 256 connections.
./scripts/load_test.sh

# Override mode / duration / concurrency.
./scripts/load_test.sh sync 60 128
./scripts/load_test.sh high-perf-mt 30 256
./scripts/load_test.sh http2 30 64

# Pin worker thread count for high-perf-mt.
HTTP_HANDLE_WORKERS=4 ./scripts/load_test.sh high-perf-mt

# Linux (in-container; needs Docker / podman / OrbStack).
podman run --rm --platform linux/arm64 \
    -v "$(pwd):/work" -w /work \
    docker.io/library/rust:1.88-slim \
    bash scripts/linux_bench.sh
```

Pre-conditions: `bombardier` on `$PATH` (`brew install bombardier` on macOS); the Linux script downloads its own Linux/arm64 binary inside the container.

## Caveats

- Single-machine loopback elides real-network effects: TCP slow-start, packet loss, asymmetric latency, NIC offload, CPU contention from the load tool itself. Treat numbers as **upper bounds** for what the server can do at the wire.
- 30-second windows are short. Long-tail latency, GC-style allocator quiescence, file-cache warm-up — none of those play out fully. A 5-minute window with separated client and server hosts would be more representative.
- The static body is 38 bytes. Larger bodies will be bandwidth-bound (or sendfile-bound for files past the threshold), not request-rate-bound, and the relative ordering between modes will likely shift.
- The Linux numbers run in a `linux/arm64` container on Apple HV — the same arm64 silicon as the macOS rows. They illustrate the OS networking-stack delta (kqueue vs epoll), not the x86_64 vs arm64 delta. Real x86_64 Linux numbers (e.g. on a managed runner) will differ in absolute terms but the relative ordering between modes should hold.
- The response cache amortises CPU work but caps at 256 entries — workloads with high path churn (more unique files than the cap can hold) will see periodic cache eviction and more uncached lookups; the worst case is the pre-cache fast path.
