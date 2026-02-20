# SLSA Provenance Policy

This repository enforces release provenance requirements with:

- `.github/workflows/release-artifacts.yml`
- `.github/workflows/slsa-verification.yml`

## Policy Requirements

- Release artifact workflow must include `actions/attest-build-provenance`.
- Workflow permissions must include:
  - `id-token: write`
  - `attestations: write`
- Artifact upload remains required for traceability.

## Enforcement

- On pull requests, `slsa-verification.yml` validates the release workflow
  configuration and fails when provenance requirements are missing.
- Manual verification remains available through
  `slsa-verification.yml` workflow dispatch.
