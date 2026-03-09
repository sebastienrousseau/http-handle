# LTS and Lifecycle Policy

This policy defines release support windows and lifecycle expectations for
`http-handle`.

## Support Tiers

- Current: Latest stable release line. Receives features, fixes, and security
  updates.
- LTS: Designated release line with extended maintenance and conservative
  change policy.
- End of Support (EoS): No new fixes; migration to supported line required.

## Lifecycle Commitments

1. A release line may be marked LTS at or after its first stable cut.
2. LTS lines receive:
   - security patches;
   - critical bug fixes;
   - compatibility updates required for supported toolchains/platforms.
3. Non-critical features are delivered only on the Current line unless
   explicitly backported.

## Deprecation Timeline

For behavior/API deprecations on supported lines:
- announce in `CHANGELOG.md`;
- document migration path in `docs/DEPRECATION_POLICY.md`;
- keep deprecation notice for at least one stable release before removal.

## Release Documentation Requirements

Each supported release line must have:
- versioned release transition notes (`docs/RELEASE_TRANSITION_vX.Y.Z.md`);
- lifecycle status in release notes (Current/LTS/EoS);
- migration guidance when support status changes.
