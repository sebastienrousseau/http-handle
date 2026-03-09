# Migration Guide

This guide tracks migration steps between `http-handle` release lines.

## 0.0.2 -> 0.0.3

### What Changed
- Expanded feature flags (for enterprise, portability, performance, and protocol
  profiles).
- Hardened CI and release governance gates.

### Migration Steps
1. Update dependency:

```toml
[dependencies]
http-handle = "0.0.3"
```

2. Review enabled feature set and opt-in only to required modules.
3. Re-run validation gates:

```bash
cargo check --all-features
cargo test --all-features
```

4. Review docs policy updates:
- `docs/DEPRECATION_POLICY.md`
- `docs/LTS_POLICY.md`
- `docs/ERRORS_AND_RECOVERY.md`

## Future Migration Entries

For each new release line, add:
- compatibility notes;
- required config/code changes;
- rollback strategy.
