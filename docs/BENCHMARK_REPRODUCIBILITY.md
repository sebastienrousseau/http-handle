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
- Starts `examples/benchmark_target.rs`.
- Waits for readiness on the configured socket.
- Fails if connection errors appear in benchmark output.
- Applies a regression threshold via `MIN_RPS`.

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
cargo build --release --example benchmark_target --features async,high-perf
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

Performance checks run in:
- `.github/workflows/perf-regression.yml`

Recommendations:
- Keep `MIN_RPS` conservative in CI to reduce false negatives.
- Use branch-specific historical baselines for stricter release gates.

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
