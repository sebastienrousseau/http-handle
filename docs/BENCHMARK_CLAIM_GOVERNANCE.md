# Benchmark Claim Governance

Performance claims for `http-handle` must be traceable to reproducible CI
artifacts.

## Required Evidence

For any benchmark claim in release notes, PRs, or docs:

1. Reference CI artifact `perf-result` (`target/perf-result.json`).
2. Reference CI artifact `perf-claim-report` (`target/perf-claim-report.md`).
3. State baseline file used (`scripts/perf/baseline-vX.Y.Z.json` preferred).

## Minimum Disclosure

- benchmark profile and features used
- measured `rps`, `p99_latency_ms`, and `rps_per_core`
- baseline thresholds and pass/fail conclusion
- commit SHA or tag

## Reproducibility Rule

Claims without associated CI artifacts are non-authoritative and must not be
used as release-quality performance statements.
