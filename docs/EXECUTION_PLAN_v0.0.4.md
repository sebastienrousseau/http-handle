# Execution Plan: v0.0.4

## Goal
- Convert `v0.0.3` foundations into enterprise-grade defaults with measurable latency/throughput/security improvements.

## Milestone Scope

### P0
- Finalize async-first serving as primary path and demote legacy sync path to compatibility mode.
- Complete end-to-end performance regression harness with CI pass/fail thresholds.
- Lock release artifact reproducibility (SLSA/SBOM/provenance) as merge blockers.
- Close all open security findings with owner + SLA tracking in workflow reports.

### P0 Gate Implementation Status (as of 2026-03-09)
- [x] Performance CI thresholds + versioned baselines:
  - `.github/workflows/perf-regression.yml`
  - `scripts/perf/benchmark_matrix.sh`
  - `scripts/perf/baseline-v0.0.3.json`
- [x] Performance claim evidence artifacts:
  - `scripts/perf/render_claim_report.sh`
  - `docs/BENCHMARK_CLAIM_GOVERNANCE.md`
- [x] Release reproducibility controls:
  - `.github/workflows/release-artifacts.yml`
  - `.github/workflows/sbom-attestation.yml`
  - `.github/workflows/slsa-verification.yml`
- [x] Security zero-high + SLA tracking:
  - `.github/workflows/security-zero-high.yml`
  - `scripts/enforce_security_sla.sh`
  - `security/findings-sla.tsv`
- [x] Aggregated release-readiness merge gate:
  - `.github/workflows/release-readiness-gate.yml`
  - `scripts/verify_execution_plan_p0.sh`

### P1
- Expand enterprise policy surface:
  - JWT/API-key/mTLS policy integration examples.
  - Structured audit pipelines with trace correlation.
- Improve portability depth:
  - Cross-target integration checks for Linux/macOS/Windows parity.
  - Extended conformance scenarios for path/socket semantics.
- Harden docs publication:
  - docs.rs + gh-pages parity checks in CI.

### P2
- Developer experience and adoption:
  - richer scenario examples and migration playbooks,
  - benchmark reproducibility packs and known-good profiles.

## KPI Targets
- p99 latency: improve by >=20% vs `v0.0.3` benchmark baseline.
- Throughput/core: improve by >=20% vs `v0.0.3` baseline.
- Security: 0 open high severity findings at merge time.
- Portability: all matrix targets green without flaky reruns.
- Docs quality: score gates stay at 100/100.

## Delivery Cadence
1. Week 1: performance path hardening + benchmark gate finalization.
2. Week 2: enterprise policy integrations + audit observability.
3. Week 3: portability expansion + release reproducibility locks.
4. Week 4: docs/tutorial polish + release candidate cut.

## Exit Criteria
- All P0 complete.
- P1 minimum viable scope complete.
- CI/verification/certification fully green.
- `CHANGELOG.md` updated and `v0.0.4` release candidate prepared.
