# Migration Guide

This guide tracks migration steps between `http-handle` release lines.

## 0.0.4 -> 0.0.5

### What Changed

**Breaking** (4 items, low blast radius — see CHANGELOG for the full list):

- `Request::headers` is now `Vec<(String, String)>` (was `HashMap<String, String>`). The `Request::header(name)` accessor is unchanged; only direct construction and iteration change shape.
- `Request::headers()` returns `&[(String, String)]` (was `&HashMap<String, String>`). Iteration via `.iter()` works as before; `.get(name)` / `.contains_key(name)` need to switch to a linear scan or `Request::header(name)`.
- `#![forbid(unsafe_code)]` at crate root replaced with `#![deny(unsafe_code)]` plus three targeted `#[allow]`s (libc::sendfile, two test-module env-var mutations). The crate-wide guarantee is preserved; downstream code that imported the lint level via reflection won't see a difference.
- `Server` gained a `canonical_document_root: PathBuf` field cached at build time. Constructing `Server` via struct-literal syntax outside `ServerBuilder` now needs `..Default::default()` for the new field. The field is `#[serde(skip)]` so the wire shape of `Server` serde output is unchanged.

**Added (highlights)**:

- HTTP/1.1 keep-alive on both server paths (sync + `start_high_perf`), 100 requests / 5 s idle per connection.
- Multi-thread async runtime: `start_high_perf_multi_thread` behind a new `high-perf-multi-thread` feature flag.
- Sharded rate limiter (16-way) and ETag LRU cache.
- Pre-serialised response cache on the high-perf static-file fast path (gated by `sendfile_threshold_bytes`, capped at 256 entries / ~16 MiB).
- 30 one-word examples mirroring `noyalib/examples`; `scripts/example.sh <name>` auto-resolves the required Cargo features.
- HTTP/3 design proposal in `docs/HTTP3_DESIGN.md` (deferred to v0.2; the `http3-profile` feature ships the ALPN routing + fallback chain).
- Linux benchmark harness: `scripts/linux_bench.sh` reproduces the Linux numbers in a `linux/arm64` container.

**Performance** (256-conn keep-alive, 30 s, small body, same Apple Silicon host):

| Mode | v0.0.4 | v0.0.5 |
|---|---|---|
| `sync` | 29,944 | 29,815 |
| `high-perf` | 9,583 | **27,971** (+192%) |
| `high-perf-mt` | 8,914 | **32,181** (+261%, beats sync) |

Linux/arm64 in container hits **180 k req/s** on `high-perf-mt`. See `docs/PERFORMANCE.md` for the full matrix.

### Migration Steps

1. Update dependency:

```toml
[dependencies]
http-handle = "0.0.5"
```

2. If you constructed `Request` via struct literal (uncommon), switch the headers field to a `Vec`:

```rust,ignore
// before (0.0.4)
let mut headers = std::collections::HashMap::new();
headers.insert("content-type".to_string(), "text/plain".to_string());
let request = Request {
    method: "GET".into(),
    path: "/".into(),
    version: "HTTP/1.1".into(),
    headers,
};

// after (0.0.5)
let request = Request {
    method: "GET".into(),
    path: "/".into(),
    version: "HTTP/1.1".into(),
    headers: vec![("content-type".into(), "text/plain".into())],
};
```

3. If you constructed `Server` via struct literal outside `ServerBuilder`, add `..Default::default()` so the new `canonical_document_root` field is populated. Recommended path: build via `Server::builder().address(...).document_root(...).build()?`.

4. Re-run validation gates:

```bash
cargo check --all-features
cargo test --all-features
```

5. Review:
   - `docs/PERFORMANCE.md` — new bench harness + numbers.
   - `docs/EXAMPLES.md` — capability → example matrix (every feature has a one-word demo).
   - `CHANGELOG.md` — full release notes.

## 0.0.3 -> 0.0.4

### What Changed
- Release publication is now anchored to the merged `main` state from PR #81.
- Documentation and release notes were synchronized for the new release tag.

### Migration Steps
1. Update dependency:

```toml
[dependencies]
http-handle = "0.0.4"
```

2. Re-run validation gates:

```bash
cargo check --all-features
cargo test --all-features
```

## 0.0.2 -> 0.0.3

### What Changed
- Expanded feature flags (for enterprise, portability, performance, and protocol
  profiles).
- Hardened CI and release governance gates.

### Migration Steps
1. Update dependency:

```toml
[dependencies]
http-handle = "0.0.3"
```

2. Review enabled feature set and opt-in only to required modules.
3. Ensure toolchain compatibility:
   - Minimum Rust version is `1.88.0` (`rust-version` in `Cargo.toml`).
   - `euxis-commons = 0.0.2` requires Rust `1.88.0`.
4. Re-run validation gates:

```bash
cargo check --all-features
cargo test --all-features
```

5. Review docs policy updates:
- `docs/DEPRECATION_POLICY.md`
- `docs/LTS_POLICY.md`
- `docs/ERRORS_AND_RECOVERY.md`

## Future Migration Entries

For each new release line, add:
- compatibility notes;
- required config/code changes;
- rollback strategy.
