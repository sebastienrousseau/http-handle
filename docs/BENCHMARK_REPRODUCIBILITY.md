# Benchmark Reproducibility

Use this guide to produce benchmark numbers you can compare across commits and environments.

## Objective

Measure:
- Throughput (`Reqs/sec`)
- Latency (`Avg`, `Max`, and tail where available)
- Error rate (`4xx`, `5xx`, transport errors)

## Benchmark Harness

Primary script:
- `scripts/perf/benchmark_matrix.sh`

What the script enforces:
- Creates deterministic static assets under `target/perf-root`.
- Starts `examples/bench.rs` (formerly `benchmark_target.rs`).
- Waits for readiness on the configured socket.
- Fails if connection errors appear in benchmark output.
- Applies regression thresholds for:
  - minimum `rps`
  - minimum `rps_per_core`
  - maximum `p99_latency_ms`
- Automatically selects `scripts/perf/baseline-v<crate-version>.json` when present.

## Environment Standard

Use this baseline for stable comparisons:
- CPU governor: performance mode where available.
- No background workload saturation.
- Same Rust toolchain and feature set for A/B runs.
- Same request mix and concurrency values.

Capture key environment details:

```bash
rustc -Vv
uname -a
sysctl -n machdep.cpu.brand_string 2>/dev/null || true
nproc 2>/dev/null || sysctl -n hw.ncpu
```

## Reproducible Run Commands

1. Build once:

```bash
cargo build --release --example bench --features 'async,high-perf,high-perf-multi-thread,http2'
```

2. Run benchmark matrix with explicit controls:

```bash
HTTP_HANDLE_MODE=high-perf \
HTTP_HANDLE_FEATURES=async,high-perf \
HTTP_HANDLE_ADDR=127.0.0.1:8090 \
BOMBARDIER_C=128 \
BOMBARDIER_REQS=20000 \
MIN_RPS=1500 \
bash scripts/perf/benchmark_matrix.sh
```

## Compare Two Commits

1. Run the command above on baseline commit and save output.
2. Run the same command on candidate commit.
3. Compare:
- `Reqs/sec` delta
- latency delta
- error count delta

Use this interpretation:
- Candidate is acceptable only if throughput improves or stays within the allowed regression threshold and no new errors appear.

## CI Integration

Performance regression checks run inside the consolidated CI pipeline (`.github/workflows/ci.yml` calls
`sebastienrousseau/pipelines/.github/workflows/rust-ci.yml` which exercises the standard test matrix).
The bombardier driver is not yet wired into CI; reproduce locally with `scripts/load_test.sh` (macOS) or
`scripts/linux_bench.sh` (Linux container).

Recommendations:
- Keep baseline thresholds conservative when wiring perf gating to reduce false negatives.
- Store version-specific baselines (`baseline-vX.Y.Z.json`) for release gating.

## Common Failure Modes

- `connection refused`: benchmark target did not start or wrong address/port.
- zero successful HTTP codes: wrong path, invalid root, or startup failure.
- unstable throughput: host contention or thermal throttling.

## Reporting Template

Use this format in PR comments:

```text
Benchmark profile: async,high-perf
Host: <cpu / cores / os>
Baseline commit: <sha> | Reqs/sec: <value> | Avg latency: <value>
Candidate commit: <sha> | Reqs/sec: <value> | Avg latency: <value>
Delta: <percent>
Errors introduced: <yes/no>
```
