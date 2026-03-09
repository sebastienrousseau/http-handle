# Deprecation and Migration Policy

This policy defines how `http-handle` communicates deprecations and guarantees
migration paths for users.

## Policy Rules

1. Every deprecation must include:
   - The version where deprecation started.
   - The planned removal version (or "TBD" if not scheduled yet).
   - A direct replacement path.
2. Deprecations must appear in:
   - API docs (`#[deprecated(note = "...")]` where applicable).
   - `CHANGELOG.md` in the release section where deprecation begins.
   - Migration examples or tutorials when usage patterns change.
3. Breaking removals must not occur without at least one prior release carrying
   the documented deprecation notice.

## Required Migration Entry Format

Use this format for each deprecation item:

- Deprecated in: `vX.Y.Z`
- Planned removal: `vA.B.C`
- Replacement: `<new API or behavior>`
- Migration steps:
  1. `<step>`
  2. `<step>`
  3. `<step>`

## Release Gate Expectations

For any release that introduces or removes deprecated behavior:
- docs quality and governance checks must pass;
- migration guidance must be included in docs updates;
- changelog must explicitly list deprecation/migration details.
