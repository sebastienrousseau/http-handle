# Multi-Tenant Isolation

Tenant-scoped configuration and secrets are available in
`src/tenant_isolation.rs`.

## Components

- `TenantConfigStore` for per-tenant config snapshots.
- `SecretProvider` trait for external secret systems.
- `EnvSecretProvider` for env-var-backed secret lookup.
- `StaticSecretProvider` for local development/testing.
- `TenantScopedSecrets<P>` for tenant-restricted access.

## Example

See: `examples/tenant.rs` (run via `./scripts/example.sh tenant`).
