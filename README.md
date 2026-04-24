<p align="center">
  <img src="https://kura.pro/http-handle/images/logos/http-handle.svg" alt="HTTP Handle logo" width="128" />
</p>

<h1 align="center">HTTP Handle</h1>

<p align="center">
  <strong>A fast and lightweight Rust library for handling HTTP requests and responses.</strong>
</p>

<p align="center">
  <a href="https://github.com/sebastienrousseau/http-handle/actions"><img src="https://img.shields.io/github/actions/workflow/status/sebastienrousseau/http-handle/ci.yml?style=for-the-badge&logo=github" alt="Build" /></a>
  <a href="https://crates.io/crates/http-handle"><img src="https://img.shields.io/crates/v/http-handle.svg?style=for-the-badge&color=fc8d62&logo=rust" alt="Crates.io" /></a>
  <a href="https://docs.rs/http-handle"><img src="https://img.shields.io/badge/docs.rs-http-handle-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" alt="Docs.rs" /></a>
  <a href="https://codecov.io/gh/sebastienrousseau/http-handle"><img src="https://img.shields.io/codecov/c/github/sebastienrousseau/http-handle?style=for-the-badge&logo=codecov" alt="Coverage" /></a>
  <a href="https://lib.rs/crates/http-handle"><img src="https://img.shields.io/badge/lib.rs-v0.0.5-orange.svg?style=for-the-badge" alt="lib.rs" /></a>
</p>

---

## Install

```bash
cargo add http-handle
```

Or add to `Cargo.toml`:

```toml
[dependencies]
http-handle = "0.0.5"
```

You need [Rust](https://rustup.rs/) 1.88.0 or later. Works on macOS, Linux, and Windows.

---

## Overview

HTTP Handle is a lightweight HTTP server library for serving static websites. Start with `Server::new` for quick prototyping, then graduate to `ServerBuilder` for production control.

- **Static file serving** with automatic MIME detection
- **Fluent builder API** for CORS, headers, and timeouts
- **Graceful shutdown** with configurable drain timeout
- **Directory traversal protection** built in

---

## Features

| | |
| :--- | :--- |
| **Static file serving** | Route requests to a document root with MIME detection |
| **ServerBuilder** | Fluent API for CORS, headers, and timeouts |
| **Graceful shutdown** | Signal-aware shutdown with configurable drain |
| **Security** | Directory traversal protection and path sanitisation |
| **Precompressed assets** | Content negotiation for br, gzip, and zstd |
| **TLS / mTLS** | TLS and mutual-TLS configuration primitives |

---

## Usage

```rust,no_run
use http_handle::Server;

fn main() -> std::io::Result<()> {
    let server = Server::new("127.0.0.1:8080", "./public");
    server.start()
}
```

---

## Development

```bash
cargo build        # Build the project
cargo test         # Run all tests
cargo clippy       # Lint with Clippy
cargo fmt          # Format with rustfmt
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, signed commits, and PR guidelines.

---

**THE ARCHITECT** \u1d2b [Sebastien Rousseau](https://sebastienrousseau.com)
**THE ENGINE** \u1d5e [EUXIS](https://euxis.co) \u1d2b Enterprise Unified Execution Intelligence System

---

## License

Dual-licensed under [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT](https://opensource.org/licenses/MIT), at your option.

<p align="right"><a href="#http-handle">Back to Top</a></p>