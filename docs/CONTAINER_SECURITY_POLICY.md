# Container Security Policy

Container security controls are enforced by:

- `.github/workflows/container-security.yml`
- `security/cve-baseline.json`

## Distroless Runtime

Release artifacts ship distroless variants:

- `docker/http-handle.Dockerfile` (`distroless-cc`)
- `docker/http-handle-static.Dockerfile` (`distroless-static`)

## CVE Drift Gating

The container security workflow:

1. Builds runtime images.
2. Scans with Trivy.
3. Compares vulnerability counts against `security/cve-baseline.json`.
4. Fails CI if any severity exceeds baseline thresholds.

## Baseline Updates

Baseline changes must be intentional and reviewed. Lowering security
requirements should include clear justification in PR notes.
