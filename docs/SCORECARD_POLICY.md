# Scorecard Threshold Policy

This project enforces OSSF Scorecard policy in CI using
`.github/workflows/scorecard.yml`.

## Policy

- All Scorecard checks reported as `error` in SARIF must be resolved.
- Temporary exceptions are allowed only via
  `.github/scorecard-exceptions.txt`.
- Exceptions must be short-lived and removed as soon as remediations ship.

## Enforcement

- The `Enforce Scorecard threshold policy` step fails the workflow if any
  non-excepted `error` checks remain.
- The Scorecard workflow is expected to be configured as a required status
  check in repository branch protection.
