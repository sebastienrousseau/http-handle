# Issue Progress (feat/v0.0.3)

This file tracks implementation progress for open roadmap issues in this branch.

## Implemented in this branch

- #71 Documentation governance and quality rubric
  - CI policy: `scripts/enforce_docs_governance.sh`
  - Workflow wiring: `.github/workflows/docs-quality.yml`
  - Scheduled audit: `.github/workflows/docs-governance-audit.yml`

- #38 API and scenario documentation completeness
  - Added: `docs/ERRORS_AND_RECOVERY.md`
  - Added: `docs/DEPRECATION_POLICY.md`
  - Updated: `docs/TUTORIALS.md`

- #69 LTS and deprecation lifecycle policy
  - Added: `docs/LTS_POLICY.md`

- #70 Developer ecosystem pack
  - Added: `docs/MIGRATION_GUIDE.md`
  - Added: `docs/RECIPES.md`

- #68 Compatibility matrix and conformance certification
  - Added gate: `scripts/validate_portability_matrix.sh`
  - Wired into `.github/workflows/portability.yml`

- #72 Benchmark transparency and claim governance
  - Added claim report generator: `scripts/perf/render_claim_report.sh`
  - Added governance doc: `docs/BENCHMARK_CLAIM_GOVERNANCE.md`
  - CI artifacts: `perf-result`, `perf-claim-report`

- #73 v0.0.4 execution-plan P0 gate coverage
  - Added verifier: `scripts/verify_execution_plan_p0.sh`
  - Added aggregate gate workflow: `.github/workflows/release-readiness-gate.yml`
  - Added execution gate workflow: `.github/workflows/execution-plan-p0.yml`
  - Added security SLA enforcement: `scripts/enforce_security_sla.sh`, `security/findings-sla.tsv`

## Remaining major epics

Large runtime/product features (proxy core, L7 routing, websocket, gRPC, authz pipeline, HTTP/3 runtime, etc.) are not complete in this branch and require dedicated implementation tracks.
