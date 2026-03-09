# Security Findings SLA Policy

Security findings are tracked in `security/findings-sla.tsv` and enforced by CI.

## Required Fields

Each finding entry must include:
- `id`
- `severity` (`LOW|MEDIUM|HIGH|CRITICAL`)
- `status` (`OPEN|IN_PROGRESS|ACCEPTED_RISK|RESOLVED`)
- `owner`
- `sla_days`
- `discovered_on`
- `last_reviewed`

## Enforcement Rules

- Any open `HIGH` or `CRITICAL` finding fails CI.
- Any finding exceeding `sla_days` fails CI.
- CI emits `target/security-sla-report.md` as an artifact for review/audit.

## Workflow Integration

The policy is enforced in:
- `.github/workflows/security-zero-high.yml`
- `scripts/enforce_security_sla.sh`
