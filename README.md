<!-- markdownlint-disable MD033 MD041 -->
<img src="https://kura.pro/http-handle/images/logos/http-handle.svg"
alt="Http Handle logo" height="66" align="right" />
<!-- markdownlint-enable MD033 MD041 -->

# HTTP Handle (http-handle)

A Rust-based HTTP server for serving static websites.

<!-- markdownlint-disable MD033 MD041 -->
<center>
<!-- markdownlint-enable MD033 MD041 -->

[![Made With Love][made-with-rust]][08] [![Crates.io][crates-badge]][03] [![lib.rs][libs-badge]][01] [![Docs.rs][docs-badge]][04] [![Codecov][codecov-badge]][06] [![Build Status][build-badge]][07] [![GitHub][github-badge]][09]

• [Website][00] • [Documentation][04] • [Report Bug][02] • [Request Feature][02] • [Contributing Guidelines][05]

<!-- markdownlint-disable MD033 MD041 -->
</center>
<!-- markdownlint-enable MD033 MD041 -->

## Architectural Overview

Use `http-handle` to serve static content fast, then scale to production policies without rewriting your core server path.

Follow this critical path:

1. Build and configure a server (`Server` / `ServerBuilder`).
2. Parse incoming HTTP requests into typed request data.
3. Generate and emit policy-aware HTTP responses.

## Feature List

- **Core Serving**: Static file routing, MIME detection, custom 404 pages, and request/response primitives.
- **Operational Safety**: Directory traversal protection, graceful shutdown, and configurable timeout handling.
- **Performance Paths**: Sync + async serving, precompressed asset negotiation (`br` / `gzip` / `zstd`), and high-performance runtime mode.
- **Protocol Growth**: HTTP/2 support and HTTP/3 profile/fallback policy primitives.
- **Enterprise Controls**: TLS/mTLS policy, API key/JWT auth hooks, RBAC/ABAC adapters, and runtime config reload patterns.
- **Scale Features**: Distributed rate limiting, tenant isolation, observability hooks, and runtime auto-tuning.

## Platform Support Matrix

`http-handle` is validated with portability conformance tests and CI coverage across Unix-like targets.

| Platform | Status | Notes |
|---|---|---|
| macOS | Supported | Primary development workflow and CI coverage. |
| Linux | Supported | Production target for deployments and containers (validated on Ubuntu CI; Debian-compatible runtime assumptions). |
| Windows (MSVC) | Supported | Tier-1 target in the portability policy and CI matrix. |
| WSL (Windows Subsystem for Linux) | Supported | Uses Linux target behavior in WSL runtime. |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
http-handle = "0.0.3"
```

## Quick Start

Start with `Server::new`. Move to `ServerBuilder` when you need explicit policy controls.

```rust,no_run
use http_handle::Server;

fn main() -> std::io::Result<()> {
    let server = Server::new("127.0.0.1:8080", "./public");
    server.start()
}
```

The server listens on `127.0.0.1:8080` and serves files from `./public`.

## Documentation

Primary API docs: [docs.rs/http-handle][04]  
GitHub Pages mirror: <https://sebastienrousseau.github.io/http-handle/>

Local docs rendering (with docs.rs feature-gate badges):

```bash
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --open --all-features
```

Supporting docs:
- Portability matrix: [`docs/PORTABILITY_MATRIX.md`](docs/PORTABILITY_MATRIX.md)
- Protocol support: [`docs/PROTOCOL_SUPPORT.md`](docs/PROTOCOL_SUPPORT.md)
- Tutorials: [`docs/TUTORIALS.md`](docs/TUTORIALS.md)
- Architecture diagrams: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Benchmark reproducibility: [`docs/BENCHMARK_REPRODUCIBILITY.md`](docs/BENCHMARK_REPRODUCIBILITY.md)
- Supply chain and scorecard policy: [`docs/SLSA_POLICY.md`](docs/SLSA_POLICY.md), [`docs/SCORECARD_POLICY.md`](docs/SCORECARD_POLICY.md)
- Container hardening: [`docs/CONTAINER_SECURITY_POLICY.md`](docs/CONTAINER_SECURITY_POLICY.md)
- Distributed rate limiting: [`docs/DISTRIBUTED_RATE_LIMITING.md`](docs/DISTRIBUTED_RATE_LIMITING.md)
- Tenant isolation: [`docs/TENANT_ISOLATION.md`](docs/TENANT_ISOLATION.md)
- Runtime auto-tuning: [`docs/RUNTIME_AUTOTUNE.md`](docs/RUNTIME_AUTOTUNE.md)
- Release transition plan: [`docs/RELEASE_TRANSITION_v0.0.3.md`](docs/RELEASE_TRANSITION_v0.0.3.md)
- Next milestone execution plan: [`docs/EXECUTION_PLAN_v0.0.4.md`](docs/EXECUTION_PLAN_v0.0.4.md)
- Changelog: [`CHANGELOG.md`](CHANGELOG.md)

## Examples

Run any example:

```shell
cargo run --example example_name
```

Start with these examples:
- `server_example`: Build a server and serve a document root.
- `server_builder_example`: Apply headers, CORS, and timeout policies.
- `feature_async_server`: Run the async accept path.
- `feature_http2_server`: Start the HTTP/2 path behind the `http2` feature.
- `feature_enterprise_authorization`: Enforce RBAC authorization for an HTTP request.
- `feature_runtime_autotune`: Derive runtime limits from host profile.

Examples index: [`docs/EXAMPLES.md`](docs/EXAMPLES.md).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the [GNU Affero General Public License v3.0][10].

## Acknowledgements

Special thanks to all contributors who have helped build the `http-handle` library.

[00]: https://http-handle.com
[01]: https://lib.rs/crates/http-handle
[02]: https://github.com/sebastienrousseau/http-handle/issues
[03]: https://crates.io/crates/http-handle
[04]: https://docs.rs/http-handle
[05]: https://github.com/sebastienrousseau/http-handle/blob/main/CONTRIBUTING.md
[06]: https://codecov.io/gh/sebastienrousseau/http-handle
[07]: https://github.com/sebastienrousseau/http-handle/actions?query=branch%3Amain
[08]: https://www.rust-lang.org/
[09]: https://github.com/sebastienrousseau/http-handle
[10]: https://www.gnu.org/licenses/agpl-3.0.en.html

[build-badge]: https://img.shields.io/github/actions/workflow/status/sebastienrousseau/http-handle/release.yml?branch=main&style=for-the-badge&logo=github
[codecov-badge]: https://img.shields.io/codecov/c/github/sebastienrousseau/http-handle?style=for-the-badge&token=OOnQTi8yIQ&logo=codecov
[crates-badge]: https://img.shields.io/crates/v/http-handle.svg?style=for-the-badge&color=fc8d62&logo=rust
[docs-badge]: https://img.shields.io/badge/docs.rs-http--handle-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
[github-badge]: https://img.shields.io/badge/github-sebastienrousseau/http--handle-8da0cb?style=for-the-badge&labelColor=555555&logo=github
[libs-badge]: https://img.shields.io/badge/lib.rs-http--handle-orange.svg?style=for-the-badge
[made-with-rust]: https://img.shields.io/badge/rust-f04041?style=for-the-badge&labelColor=c0282d&logo=rust
