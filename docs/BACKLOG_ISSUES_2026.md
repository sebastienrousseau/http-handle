# 2026 Backlog (Tracked as GitHub Issues)

The following items are intentionally deferred from the current implementation batch and should remain tracked as GitHub issues:

1. HTTP/3 production profile (QUIC tuning, ALPN routing, and fallback strategy). Issue: #21
2. Supply-chain attestation enforcement (SLSA provenance verification gate). Issue: #22
3. SBOM attestation publishing and policy validation in release pipeline. Issue: #23
4. OSSF Scorecard threshold enforcement on pull requests. Issue: #24
5. Distroless runtime hardening with CVE drift gating. Issue: #25
6. Fine-grained authorization policies (RBAC/ABAC with policy engine integration). Issue: #26
7. Distributed rate limiting backend support (Redis/memcached adapters). Issue: #27
8. Multi-tenant config isolation and secret provider integrations. Issue: #28
9. End-to-end protocol fuzzing for HTTP/2 + TLS handshake state machine. Issue: #29
10. Performance auto-tuning based on runtime host resources. Issue: #30

## Status Snapshot

- Implemented on `feat/v0.0.3` and pending issue closure verification:
  - #22 Supply-chain attestation enforcement (SLSA provenance gate)
  - #24 OSSF Scorecard threshold enforcement on pull requests
  - #26 Fine-grained authorization policies (RBAC/ABAC policy adapters)
