# Performance — External Load Numbers

External `bombardier` load test against the `benchmark_target` example. Driver: `scripts/load_test.sh`. Captured 2026-04-27 on Apple Silicon (Darwin 25.4.0), 8 cores, single-machine loopback.

These are end-to-end numbers from an external HTTP client driving real TCP keep-alive load — complementary to the criterion in-process benches, which measure single-iter cost in isolation.

## Headline (256-connection keep-alive, 15 s, small static body)

| Mode | Req/s avg | Lat p50 | Lat p90 | Lat p99 | 4xx/5xx |
|---|---|---|---|---|---|
| `sync` (`Server::start`) | **29,944** | 8.25 ms | 12.67 ms | 27.56 ms | 0 / 0 |
| `high-perf` (`start_high_perf`) | 10,626 | 23.02 ms | 27.93 ms | 39.35 ms | 0 / 0 |

Both modes ran zero errors across the test window (447 k requests for sync, 159 k for high-perf).

## Why does sync beat high-perf here?

The `benchmark_target` example builds its tokio runtime with `Builder::new_current_thread()` — single-threaded async. Under 256 concurrent keep-alive connections, all of those connections cooperatively share **one** OS thread for the async accept loop, request parse, response build, and write paths. The sync server spawns 256 OS threads; macOS's scheduler distributes them across all 8 cores in parallel.

This is a runtime-config artefact, not a defect in `perf_server`:

- **Tokio's `rt-multi-thread` feature is not currently enabled** in this crate's `tokio` dependency declaration. Switching the benchmark target to `new_multi_thread()` would require adding `rt-multi-thread` to the feature list and exposes whatever upstream cost that brings in.
- **Single-threaded async wins under different workloads.** Lots of small short-lived connections (no keep-alive), high I/O wait per request (DB calls), or memory-constrained hosts where 256 OS threads is not affordable — those favour the async path.
- **Real production deployments behind a load balancer** usually pin one process per CPU and run `current_thread` per process anyway. The sync-thread-per-connection model doesn't horizontal-scale that pattern as cleanly.

The takeaway: pick the mode that matches your deployment shape, not the bench winner. v0.0.5 ships both.

## Re-running the harness

```bash
# Default: high-perf, 30 s, 256 connections.
./scripts/load_test.sh

# Override mode / duration / concurrency.
./scripts/load_test.sh sync 60 128
./scripts/load_test.sh http2 30 64
```

Pre-conditions: `bombardier` on `$PATH` (`brew install bombardier` on macOS).

## Caveats

- Single-machine loopback elides real-network effects: TCP slow-start, packet loss, asymmetric latency, NIC offload, CPU contention from the load tool itself. Treat numbers as **upper bounds** for what the server can do at the wire.
- 15 s windows are short. Long-tail latency, GC-style allocator quiescence, file-cache warm-up — none of those play out fully. A 5-minute window with separated client and server hosts would be more representative.
- The static body is 38 bytes. Larger bodies will be bandwidth-bound (or sendfile-bound for files past the threshold), not request-rate-bound, and the relative ordering between modes will likely shift.
- These numbers are from a single hardware/OS combination. Linux numbers tend to be higher in absolute terms (especially on the sync path where Linux's `epoll`-style accept handling is more efficient than Darwin's `kqueue` fallback).
