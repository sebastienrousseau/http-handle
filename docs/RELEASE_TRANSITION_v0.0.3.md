# Release Transition Plan: v0.0.3

## Objective
- Cut and publish `v0.0.3` from `main` after PR #81 (`feat/v0.0.3`) merges with all gates green.

## Preconditions (Must Be Green)
- PR #81 merged into `main`.
- GitHub required checks passing on merge commit.
- No open Dependabot alerts for the repository.
- No open code-scanning alerts scoped to the release commit.
- `cargo test --all-features` and coverage gates passing in CI.

## Tagging Plan
1. Ensure local `main` is current:
   - `git checkout main`
   - `git pull --ff-only origin main`
2. Create annotated release tag:
   - `git tag -a v0.0.3 -m "release: v0.0.3"`
3. Push the tag:
   - `git push origin v0.0.3`

## Release Artifact Plan
1. Verify release workflow runs on tag push.
2. Validate generated artifacts:
   - Linux MUSL binary
   - OCI image variants
   - SBOM + provenance attestations
3. Confirm docs publication:
   - `docs.rs` build
   - GitHub Pages rustdoc mirror

## Changelog Cut
- Source: `CHANGELOG.md` section `[0.0.3] - 2026-02-22`.
- Use the same section as release notes body.

## Post-Release Validation
- Smoke test install path and startup on:
  - Linux
  - macOS
  - Windows (CI validation)
- Confirm benchmark regression gates remain green.

## Rollback Plan
- If release artifacts or runtime validation fail:
  - Mark GitHub release as pre-release or draft rollback notice.
  - Revert offending commit(s) on `main`.
  - Retag with `v0.0.4-rc1` after fixes instead of force-moving `v0.0.3`.
