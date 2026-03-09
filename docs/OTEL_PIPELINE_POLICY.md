# OpenTelemetry Pipeline Policy

`http-handle` telemetry export is designed to be production-safe and non-fatal.

## Runtime Behavior

- Telemetry can be enabled/disabled via policy (`TelemetryPolicy`).
- Sampling is enforced by `sample_percent` (`0..=100`).
- Schema validation rejects malformed telemetry events.
- Export endpoint failures are reported but do not interrupt request handling.

## Core Types

- `TelemetryPolicy`
- `TelemetryPipeline`
- `TelemetryExportOutcome`
- `AccessAuditEvent`

## Acceptance Guarantees

1. Enabled telemetry emits correlated records.
2. Sampling policy controls export volume.
3. Unavailable endpoints fail open (service remains stable).
4. Schema drift is surfaced via explicit failed outcomes.
5. Disabled telemetry produces no export side effects.
