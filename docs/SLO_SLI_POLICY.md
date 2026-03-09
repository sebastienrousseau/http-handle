# SLO/SLI and Error-Budget Release Policy

Release promotion is gated by SLO/SLI thresholds and error-budget health.

## Inputs

- Gate configuration: `slo/release-gate.json`
- Measured SLI metrics: `slo/current-sli.json`
- Enforcement script: `scripts/enforce_slo_gate.sh`

## Gate Rules

1. Promotion is allowed only when all SLI dimensions satisfy thresholds.
2. Promotion is blocked when error budget is exhausted or below minimum.
3. Failed gates must report impacted SLI dimensions.
4. Invalid gate configuration (out-of-range thresholds) is rejected.
5. Blocked gate output must include remediation guidance.

## CI Wiring

- `.github/workflows/slo-release-gate.yml`
- `.github/workflows/release-readiness-gate.yml`

CI uploads `target/slo-gate-report.md` as a release-gate evidence artifact.
