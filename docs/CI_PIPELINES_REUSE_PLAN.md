# CI Reuse Plan (`http-handle` -> `pipelines`)

This document defines how to extract shared CI logic into reusable workflows
under `sebastienrousseau/pipelines` to reduce duplication and maintenance.

## Current Local Reuse

- Shared composite action:
  - `.github/actions/rust-toolchain-cache/action.yml`
  - Used by:
    - `.github/workflows/perf-regression.yml`
    - `.github/workflows/docs-api-surface-score.yml`
    - `.github/workflows/docs-gh-pages.yml`

## Recommended Reusable Workflows in `pipelines`

1. `rust-setup.yml`
- Inputs: `toolchain`, `cache`, `targets`, `components`
- Responsibility: checkout + toolchain + rust-cache + target install

2. `rust-test-matrix.yml`
- Inputs: `feature_matrix`, `msrv`, `clippy`, `doc_tests`
- Responsibility: CI / feature-matrix / msrv / nightly checks

3. `docs-quality.yml`
- Inputs: `strict_score_script`, `api_surface_script`, `publish_gh_pages`
- Responsibility: docs-score + docs-api-surface + docs publish

4. `perf-regression.yml`
- Inputs: `min_rps`, `bench_script`, `server_features`
- Responsibility: benchmark bootstrap + regression threshold enforcement

5. `container-security.yml`
- Inputs: `dockerfiles`, `baseline_file`, `severity`
- Responsibility: image build + trivy scan + baseline drift enforcement

## Migration Sequence

1. Move `.github/actions/rust-toolchain-cache` into `pipelines` as either:
- shared composite action, or
- `workflow_call` reusable workflow.

2. Switch each `http-handle` workflow from local steps to:
- `uses: sebastienrousseau/pipelines/.github/workflows/<workflow>.yml@vX.Y.Z`

3. Keep repository-specific scripts local:
- `scripts/score_docs.sh`
- `scripts/score_docs_api_surface.sh`
- `scripts/perf/benchmark_matrix.sh`

4. Pin `pipelines` reusable workflow refs to immutable tags.

5. Add a single status dashboard in `pipelines` to track all app repos.

## Cost and Speed Improvements Implemented

- Removed `go install ...@latest` style behavior from perf workflow.
- Added prebuild step and prebuilt benchmark binary mode for perf gate.
- Added stronger startup readiness controls for perf harness.
- Pinned previously unpinned workflow actions in docs workflows.
- Added timeouts to docs/perf jobs to cap runaway cost.

