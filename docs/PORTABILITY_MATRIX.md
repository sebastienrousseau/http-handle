# Portability Support Matrix

## Rust Target Tier Policy

This project tracks Rust platform tiers and validates core functionality across:

- Tier 1: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Tier 2 (with host tools): `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`

## CI Validation Modes

- `check`: compile + lint + tests on host platforms
- `cross-check`: cross-compilation for additional targets
- `conformance`: path, socket, and line-ending conformance tests

## Release Artifact Targets

- Linux static binary: `x86_64-unknown-linux-musl`
- Container image: OCI-compatible image based on distroless runtime

## Operational Expectations

- No path traversal behavior differences across platforms
- Socket bind/port-zero behavior must remain stable
- Request parsing remains tolerant for CRLF and LF line endings
