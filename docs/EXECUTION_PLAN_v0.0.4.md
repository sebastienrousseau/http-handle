# Execution Plan: v0.0.4

## Goal
- Convert `v0.0.3` foundations into enterprise-grade defaults with measurable latency/throughput/security improvements.

## Milestone Scope

### P0
- Finalize async-first serving as primary path and demote legacy sync path to compatibility mode.
- Complete end-to-end performance regression harness with CI pass/fail thresholds.
- Lock release artifact reproducibility (SLSA/SBOM/provenance) as merge blockers.
- Close all open security findings with owner + SLA tracking in workflow reports.

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

