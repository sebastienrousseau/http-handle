# Performance — External Load Numbers

External `bombardier` load test against the `benchmark_target` example. Driver: `scripts/load_test.sh`. Captured 2026-04-27 on Apple Silicon (Darwin 25.4.0), 8 cores, single-machine loopback.

These are end-to-end numbers from an external HTTP client driving real TCP keep-alive load — complementary to the criterion in-process benches, which measure single-iter cost in isolation.

## Headline (256-connection keep-alive, 30 s, small static body)

| Mode | Req/s avg | Lat p50 | Lat p90 | Lat p99 | 4xx/5xx |
|---|---|---|---|---|---|
| `sync` (`Server::start`) | **29,836** | 8.30 ms | 12.79 ms | 26.06 ms | 0 / 0 |
| `high-perf-mt` (`start_high_perf_multi_thread`, default workers) | 22,510 | 10.74 ms | 13.87 ms | 23.19 ms | 0 / 0 |
| `high-perf` (`start_high_perf`, current-thread) | 14,481 | 17.90 ms | 18.34 ms | 19.98 ms | 0 / 0 |

All modes ran zero errors across the test window (892 k requests for sync, 675 k high-perf-mt, 434 k high-perf).

## Fast-path optimisation (v0.0.5)

The async path previously routed every static-file request through `tokio::fs::canonicalize` × 2, `tokio::fs::metadata` × 2, `tokio::fs::File::open`, and `tokio::io::copy`. Each one is a `spawn_blocking` round-trip — a thread context switch through tokio's blocking pool. For sub-`sendfile_threshold_bytes` files on local disk, the syscall returns in microseconds, so the round-trip dominated.

v0.0.5 keeps `tokio::fs` for the above-threshold (sendfile-fallback) path, but the small-file fast path now uses `std::fs::canonicalize` / `metadata` / `read` directly inside the async handler, then a single `write_all` of the buffered body. That removes 5 cross-thread hops per request.

Impact on this benchmark:

- `high-perf` (current-thread): **9,583 → 14,481 req/s (+51%)**, p99 51 ms → **20 ms**.
- `high-perf-mt` (multi-thread): **8,914 → 22,510 req/s (+153%)**, p99 77 ms → **23 ms**.

Multi-thread now beats single-thread, as it should: once the per-request work is no longer dominated by a blocking-pool hop, tokio's work-stealing scheduler can spread real CPU work across cores.

## Why does sync still edge ahead on this benchmark?

The hot path of `benchmark_target` is essentially "accept → parse 38-byte response → write back" — there is no I/O wait per request. Sync `Server::start` runs that straight-line on a dedicated OS thread; multi-thread async still pays a fixed reactor-poll overhead per future and contends on the inflight `Semaphore`. With the spawn_blocking hops removed, the gap is now ~1.32× (was 3.27×).

The remaining gap is workload shape, not a defect:

- **`high-perf-mt` will beat sync on mixed I/O.** When request handlers do real async work (await on a backend, run a slow downstream, multiplex across many sockets), the multi-thread runtime spreads that work across cores while sync blocks an OS thread per connection.
- **`high-perf` (current-thread) is the right primitive for memory-constrained hosts** where 256 OS threads is not affordable, or for one-process-per-CPU deployments behind a load balancer.
- **Sync wins for pure static-file serving** on memory-rich hosts where the OS scheduler can spread hundreds of threads across cores.

The takeaway: pick the mode that matches your deployment shape. v0.0.5 ships all three (`sync`, `high-perf`, `high-perf-mt`) so you can pick.

## Re-running the harness

```bash
# Default: high-perf, 30 s, 256 connections.
./scripts/load_test.sh

# Override mode / duration / concurrency.
./scripts/load_test.sh sync 60 128
./scripts/load_test.sh high-perf-mt 30 256
./scripts/load_test.sh http2 30 64

# Pin worker thread count for high-perf-mt.
HTTP_HANDLE_WORKERS=4 ./scripts/load_test.sh high-perf-mt
```

Pre-conditions: `bombardier` on `$PATH` (`brew install bombardier` on macOS).

## Caveats

- Single-machine loopback elides real-network effects: TCP slow-start, packet loss, asymmetric latency, NIC offload, CPU contention from the load tool itself. Treat numbers as **upper bounds** for what the server can do at the wire.
- 15 s windows are short. Long-tail latency, GC-style allocator quiescence, file-cache warm-up — none of those play out fully. A 5-minute window with separated client and server hosts would be more representative.
- The static body is 38 bytes. Larger bodies will be bandwidth-bound (or sendfile-bound for files past the threshold), not request-rate-bound, and the relative ordering between modes will likely shift.
- These numbers are from a single hardware/OS combination. Linux numbers tend to be higher in absolute terms (especially on the sync path where Linux's `epoll`-style accept handling is more efficient than Darwin's `kqueue` fallback).
