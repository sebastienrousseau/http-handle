# Error Causes and Recovery Guide

This guide documents common `http-handle` failure modes, likely causes, and
recommended recovery actions.

## Startup and Bind Failures

### Symptom
- Server fails to start.
- Error resembles `Address already in use` or `Permission denied`.

### Likely Causes
- Another process is already bound to the target host/port.
- Privileged port usage without required privileges.
- Invalid bind address configuration.

### Recovery
1. Check active listeners (`lsof -i :8080` or equivalent).
2. Change bind address/port to an available endpoint.
3. Use non-privileged ports (`>=1024`) unless explicitly required.

## File Serving Errors

### Symptom
- Requests return `404 Not Found` or equivalent custom error page.
- Static assets are unexpectedly missing.

### Likely Causes
- Incorrect document root.
- Files were not deployed to runtime container/image.
- Path mismatch between local and deployment environment.

### Recovery
1. Confirm configured document root exists in runtime filesystem.
2. Validate deployment artifact includes expected static assets.
3. Use absolute, environment-resolved paths for production configs.

## Timeout and Slow Request Behavior

### Symptom
- Requests fail due to timeout policy.
- Slow clients see partial or aborted transfers.

### Likely Causes
- Timeout settings are too aggressive for workload.
- Upstream or network path latency regression.

### Recovery
1. Increase request timeout in `ServerBuilder` policy.
2. Benchmark and profile under representative latency/load.
3. Use `high-perf` and async runtime features for sustained concurrency.

## TLS/mTLS Policy Misconfiguration

### Symptom
- TLS handshake failures.
- Clients rejected unexpectedly in mTLS mode.

### Likely Causes
- Certificate/key mismatch.
- Missing or incorrect trust chain.
- Client certificate not trusted by server policy.

### Recovery
1. Validate certificate/key pairing and expiration windows.
2. Ensure CA bundle and chain order are correct.
3. Verify mTLS trust anchors and client cert issuance path.

## AuthN/AuthZ Policy Rejections

### Symptom
- Requests fail authorization checks.
- API key/JWT-based paths deny known clients.

### Likely Causes
- Missing credentials or malformed token.
- Policy rules mismatch expected tenant/role claims.
- Token validation clock skew and expiry issues.

### Recovery
1. Confirm credentials are present and correctly formatted.
2. Validate claim mapping against current policy rules.
3. Check time synchronization for token expiry validation.

## Documentation/Behavior Mismatch

### Symptom
- Users follow docs and see unexpected runtime behavior.

### Recovery
1. Treat mismatch as release blocker.
2. Update docs and code in the same PR.
3. Re-run docs governance and quality gates before merge.
