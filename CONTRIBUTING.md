# Contributing to `HTTP Handle`

Welcome! We're thrilled that you're interested in contributing to the `HTTP Handle` library. Whether you're looking to evangelize, submit feedback, or contribute code, we appreciate your involvement in making `HTTP Handle` a better tool for everyone. Here's how you can get started.

## Evangelize

One of the simplest ways to help us out is by spreading the word about `HTTP Handle`. We believe that a bigger, more involved community makes for a better framework, and that better frameworks make the world a better place. If you know people who might benefit from using `HTTP Handle`, please let them know!

## How to Contribute

If you're interested in making a more direct contribution, there are several ways you can help us improve `HTTP Handle`. Here are some guidelines for submitting feedback, bug reports, and code contributions.

### Feedback

Your feedback is incredibly valuable to us, and we're always looking for ways to make `HTTP Handle` better. If you have ideas, suggestions, or questions about `HTTP Handle`, we'd love to hear them. Here's how you can provide feedback:

- Click [here][02] to submit a new feedback.
- Use a descriptive title that clearly summarizes your feedback.
- Provide a detailed description of the issue or suggestion.
- Be patient while we review and respond to your feedback.

### Bug Reports

If you encounter a bug while using `HTTP Handle`, please let us know so we can fix it. Here's how you can submit a bug report:

- Click [here][02] to submit a new issue.
- Use a descriptive title that clearly summarizes the bug.
- Provide a detailed description of the issue, including steps to reproduce it.
- Be patient while we review and respond to your bug report.

### Code Contributions

If you're interested in contributing code to `HTTP Handle`, we're excited to have your help! Here's what you need to know:

#### Feature Requests

If you have an idea for a new feature or improvement, we'd love to hear it. Here's how you can contribute code for a new feature to `HTTP Handle`:

- Fork the repo.
- Clone the [HTTP Handle][01] repo by running:
  `git clone https://github.com/sebastienrousseau/http-handle.git`
- Edit files in the `src/` folder. The `src/` folder contains the source code for `HTTP Handle`.
- Submit a pull request, and we'll review and merge your changes if they fit with our vision for `HTTP Handle`.

#### Submitting Code

If you've identified a bug or have a specific code improvement in mind, we welcome your pull requests. Here's how to submit your code changes:

- Fork the repo.
- Clone the `HTTP Handle` repo by running:
  `git clone https://github.com/sebastienrousseau/http-handle.git`
- Edit files in the `src/` folder. The `src/` folder contains the source code for `HTTP Handle`.
- Submit a pull request, and we'll review and merge your changes if they fit with our vision for `HTTP Handle`.

We hope that this guide has been helpful in explaining how you can contribute to `HTTP Handle`. Thank you for your interest and involvement in our project!

## Development Workflow

### Build, lint, test

```bash
cargo fmt --check                # rustfmt; CI-enforced
cargo clippy --all-targets -- -D warnings
cargo test --features "async,auth,batch,config,distributed-rate-limit,enterprise,env_logger,high-perf,http2,http3-profile,multi-tenant,autotune,observability,optimized,streaming,tls" --lib
```

The full feature matrix above is what CI runs. Skipping `multi-tenant` and `autotune` reproduces the historical `forbid(unsafe_code)` build failures — those features now build cleanly under the v0.0.5 `deny + targeted allow` policy.

### Supply-chain audit

```bash
cargo deny check                 # licenses, advisories, bans, sources
cargo audit --deny warnings      # rustsec advisories against Cargo.lock
```

`deny.toml` carries the four-category policy. `Cargo.lock` is committed so both checks are reproducible. New dependencies must be flagged in the PR description with rationale per the project's no-bloat policy.

### Benchmarks

The project ships four `criterion` bench targets:

| Target | Scope | Features |
|---|---|---|
| `server_benchmark` | Sync server, thread-pool, rate-limit-on, shutdown-aware | default |
| `perf_server_benchmark` | Async `start_high_perf` single + concurrent fan-outs | `high-perf` |
| `http2_benchmark` | h2c single-stream regression baseline | `http2` |
| `micro_benchmark` | Sub-µs in-process (header lookup, response Connection) | default |

```bash
cargo bench --bench server_benchmark
cargo bench --bench perf_server_benchmark --features high-perf
cargo bench --bench micro_benchmark
```

For external load testing, `scripts/load_test.sh` drives `bombardier` against the `bench` example and prints throughput / latency distribution. Pre-condition: `bombardier` on `$PATH`. Linux numbers for `docs/PERFORMANCE.md` are reproduced via `scripts/linux_bench.sh` inside a `linux/arm64` container (`podman run --platform linux/arm64 -v "$(pwd):/work" -w /work docker.io/library/rust:1.88-slim bash scripts/linux_bench.sh`).

### Heap profiling

```bash
cargo run --release --example dhat --features high-perf
# Writes dhat-heap.json. View with https://nnethercote.github.io/dh_view/dh_view.html.
```

Findings from the v0.0.5 baseline run are documented in [docs/HEAP_PROFILE.md](docs/HEAP_PROFILE.md).

### Performance baselines

Latest measured numbers from criterion micro/integration benches and external `bombardier` load tests are kept in [docs/PERFORMANCE.md](docs/PERFORMANCE.md). Update those when a perf-affecting change lands.

### Dev dependency footprint

Three dev-deps were added in v0.0.5; they are dev-only and never linked into production builds:

- `dhat = "0.3"` — heap profile harness (`examples/dhat.rs`).
- `criterion` — bench harness (existed pre-0.0.5; documented here for completeness).
- `proptest`, `assert_fs`, `predicates`, `tempfile` — test fixtures.

Two regular dependencies were added:

- `crossbeam-channel = "0.5"` — lock-free MPMC channel for the `ThreadPool` worker queue. Widely deployed, ~50 KB compiled.
- `memchr = "2.8"` — SIMD `:` separator search in HTTP header parsing. Already transitively present via `regex`/`aho-corasick`; declaring it directly costs zero additional compile time.

[01]: https://github.com/sebastienrousseau/http-handle
[02]: https://github.com/sebastienrousseau/http-handle/issues/new
