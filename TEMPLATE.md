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

## Overview

The `http-handle` is a robust Rust library designed for serving static websites. It provides a simple yet efficient HTTP server implementation with features like request parsing, response generation, and basic security measures. The library is not intended to be a full-fledged web server but rather a lightweight solution for serving static files over HTTP for development and testing purposes.

## Features

- **Static File Serving**: Serve static files from a configured document root.
- **Request Parsing**: Parse incoming HTTP requests with proper error handling.
- **Response Generation**: Generate appropriate HTTP responses based on requests.
- **Security Measures**: Prevent directory traversal attacks.
- **Content Type Detection**: Automatically detect and set appropriate content types for files.
- **Customizable 404 Handling**: Support for custom 404 error pages.
- **Threaded Connections**: Handle multiple connections concurrently using threads.
- **Configurable Server**: Easy configuration of server address and document root.

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

[build-badge]: https://img.shields.io/github/actions/workflow/status/sebastienrousseau/http-handle/release.yml?branch=main&style=for-the-badge&logo=github
[codecov-badge]: https://img.shields.io/codecov/c/github/sebastienrousseau/http-handle?style=for-the-badge&token=your_token_here&logo=codecov
[crates-badge]: https://img.shields.io/crates/v/http-handle.svg?style=for-the-badge&color=fc8d62&logo=rust
[docs-badge]: https://img.shields.io/badge/docs.rs-http--handle-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
[github-badge]: https://img.shields.io/badge/github-sebastienrousseau/http--handle-8da0cb?style=for-the-badge&labelColor=555555&logo=github
[libs-badge]: https://img.shields.io/badge/lib.rs-v0.0.1-orange.svg?style=for-the-badge
[made-with-rust]: https://img.shields.io/badge/rust-f04041?style=for-the-badge&labelColor=c0282d&logo=rust

## Changelog 📚
