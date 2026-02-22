# SLSA Provenance Policy

This repository enforces release provenance requirements with:

- `.github/workflows/release-artifacts.yml`
- `.github/workflows/slsa-verification.yml`

## Policy Requirements

- Release artifact and SBOM workflows must include
  `actions/attest-build-provenance`.
- Workflow permissions must include, at top level:
  - `id-token: write`
  - `attestations: write`
- Artifact upload remains required for traceability.
- All workflow actions in release-related workflows must be pinned to full
  commit SHAs.

## Enforcement

- On pull requests, `slsa-verification.yml` validates both release workflows and
  fails when provenance requirements or SHA pinning are missing.
- Manual verification remains available through
  `slsa-verification.yml` workflow dispatch.
