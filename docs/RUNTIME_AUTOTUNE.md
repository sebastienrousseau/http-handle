# Runtime Auto-Tuning

Runtime tuning helpers are implemented in `src/runtime_autotune.rs`.

## Flow

1. Detect host profile (`cpu_cores`, `memory_mib`).
2. Generate recommendation with `RuntimeTuneRecommendation::from_profile`.
3. Convert to high-performance limits when `high-perf` is enabled.

## Integration

`examples/bench.rs` supports:

- `HTTP_HANDLE_MODE=high-perf` (or `high-perf-mt`)
- `HTTP_HANDLE_AUTOTUNE=1`
- `HTTP_HANDLE_WORKERS=N` (multi-thread mode only)

to enable host-profile-derived tuning in benchmark runs. See also the
focused `autotune` example: `./scripts/example.sh autotune`.
