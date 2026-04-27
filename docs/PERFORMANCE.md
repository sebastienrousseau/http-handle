# Performance — External Load Numbers

External `bombardier` load test against the `benchmark_target` example. Driver: `scripts/load_test.sh`. Captured 2026-04-27 on Apple Silicon (Darwin 25.4.0), 8 cores, single-machine loopback.

These are end-to-end numbers from an external HTTP client driving real TCP keep-alive load — complementary to the criterion in-process benches, which measure single-iter cost in isolation.

## Headline (256-connection keep-alive, 30 s, small static body)

| Mode | Req/s avg | Lat p50 | Lat p90 | Lat p99 | 4xx/5xx |
|---|---|---|---|---|---|
| `sync` (`Server::start`) | **29,164** | 7.63 ms | 14.80 ms | 35.94 ms | 0 / 0 |
| `high-perf` (`start_high_perf`, current-thread) | 9,583 | 25.67 ms | 30.45 ms | 51.32 ms | 0 / 0 |
| `high-perf-mt` (`start_high_perf_multi_thread`, default workers) | 8,914 | 26.24 ms | 39.67 ms | 77.45 ms | 0 / 0 |
| `high-perf-mt` (workers = 4) | 5,991 | 33.13 ms | 70.59 ms | 182.44 ms | 0 / 0 |

All modes ran zero errors across the test window (867 k requests for sync, 287 k high-perf, 267 k high-perf-mt default).

## Why does sync beat the async modes on this benchmark?

The hot path of `benchmark_target` is essentially "accept → parse 38-byte response → write back" — there is no I/O wait per request. Under those conditions the per-future polling overhead of tokio's reactor outweighs anything async buys you, and the sync `Server::start` thread-per-connection model just runs the request handler straight-line on whichever core macOS scheduled the OS thread on.

Adding tokio's multi-thread runtime (`high-perf-mt`) does **not** close the gap on this benchmark. With 8 worker threads it lands a touch below the current-thread variant; constraining to 4 workers makes it worse. The dominant cost under high concurrent keep-alive on a CPU-bound static workload is contention on the inflight `Semaphore` and cross-core scheduling overhead — neither of which scales by adding workers.

This is workload-shape, not a defect in `perf_server`:

- **Single-threaded async wins under different workloads.** Lots of small short-lived connections (no keep-alive), high I/O wait per request (DB / upstream calls), or memory-constrained hosts where 256 OS threads is not affordable — those favour the async path.
- **`high-perf-mt` is the right primitive for mixed I/O.** When request handlers do real async work (await on a backend, run a slow downstream, multiplex across many sockets), the multi-thread runtime spreads that work across cores. On a pure static-file hot path it's overhead.
- **Real production deployments behind a load balancer** usually pin one process per CPU and run `current_thread` per process anyway. The sync-thread-per-connection model doesn't horizontal-scale that pattern as cleanly.

The takeaway: pick the mode that matches your deployment shape, not the bench winner. v0.0.5 ships all three (`sync`, `high-perf`, `high-perf-mt`) so you can pick.

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
