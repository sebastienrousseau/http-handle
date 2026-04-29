# Heap Profile Findings — `dhat` example

Captured 2026-04-27 via `cargo run --example dhat --features high-perf` against the v0.0.5 `feat/v0.0.5` branch on Apple Silicon (Darwin 25.4.0).

Workload: 1024 sequential GET roundtrips with `Connection: close` against `start_high_perf` on a current-thread tokio runtime.

## Headline numbers

| Metric | Value |
|---|---|
| Total bytes allocated | 40,130,345 B (≈ 40 MB) |
| Total allocation calls | 45,155 |
| Peak resident (`t-gmax`) | 121,164 B (≈ 121 KB) |
| Resident at end (`t-end`) | 43,996 B (≈ 44 KB) |
| **Average per request** | **~39 KB allocated, ~118 B peak resident** |

The 1000× ratio between total-allocated and peak-resident is the green light: every per-request allocation is paired with a deallocation before the next request, so the server doesn't accumulate memory under sustained load.

## Top allocation sites

| # | % of total | Bytes | Blocks | Site |
|---|---|---|---|---|
| 1 | **41.85 %** | 16,793,600 | 1,025 | `perf_server::handle_async_connection:182` — per-iter 16 KiB read buffer |
| 2 | 20.90 % | 8,388,608 | 1,024 | `perf_server::try_send_static_file_fast_path:422` — `tokio::io::copy` internal buffer (8 KiB) |
| 3 | 20.90 % | 8,388,608 | 1,024 | tokio internal (also 8 KiB × 1024 — paired buffer) |
| 4 | 3.92 % | 1,574,400 | 1,025 | tokio internal |
| 5 | 1.96 % | 786,432 | 2,048 | tokio internal |
| 6–7 | 2.62 % | ~1 MB | ~4,000 | tokio internal |
| 8 | 0.98 % | 393,216 | 1,024 | `perf_server::parse_request_from_bytes:263` — header `Vec<(String, String)>` push allocs |
| 9 | 0.75 % | 301,056 | 2,048 | `try_send_static_file_fast_path:386` — header String build buffer |

Sites attributed to `http_handle::*`: **64 % of total bytes**. The rest is tokio's read/write infrastructure (kernel reads, file copy buffers, runtime task scheduling) which is amortised across hundreds of thousands of requests in real workloads.

## Action taken in v0.0.5

- **Site #1 (16 KiB read buffer)** — hoisted out of the keep-alive loop in `handle_async_connection`. Under `Connection: close` benches the count stays at one alloc per connection (no compounding), but under HTTP/1.1 keep-alive the buffer is now reused across all requests on a connection — saves 16 KiB × N where N is requests-per-connection.
- **Sites #2 / #3** (`tokio::io::copy` 8 KiB buffer ×2) are tokio internals; the `streaming::ChunkStream` path will replace this with a smaller bounded buffer once `ResponseBody` lands in v0.1.
- **Site #8** (header push allocs) is a known cost of the `Vec<(String, String)>` model from P1.A. A future `Vec<(Cow<'static, str>, String)>` with interned common header names could halve this.
- **Site #9** (response head String) is bounded; a `String::with_capacity` + `write!` swap is in scope but below the noise floor of the benchmark suite.

## How to re-run

```bash
cargo run --release --example dhat --features high-perf
# or, via the wrapper:
./scripts/example.sh dhat --release
```

The profile writes `dhat-heap.json` in the working directory. View interactively with [`dh_view.html`](https://nnethercote.github.io/dh_view/dh_view.html). Use the **dev** profile (debug symbols preserved) — the release profile strips, leaving frames as raw addresses.

## Caveats

- Workload uses `Connection: close` so the server's keep-alive loop runs exactly once per connection. The 16 KiB read-buffer hoist is correct but not visible in the totals here; under a pipelined load harness it would compound proportionally to keep-alive depth.
- 1024 iterations is short — long-tail allocation patterns (cache evictions, semaphore queue growth, etc.) won't show up. A second profile at 100k iterations would surface those.
- `dhat::Profiler` adds its own bookkeeping allocations to every malloc/free; relative ordering is reliable but absolute totals include the instrumentation cost.
