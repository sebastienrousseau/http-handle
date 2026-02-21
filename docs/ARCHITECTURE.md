# Architecture Diagrams

Use this page to understand request flow, module boundaries, and feature layering.

## Request Lifecycle

```mermaid
flowchart LR
    A[Client Connection] --> B[Accept Loop]
    B --> C[Request Parser]
    C --> D[Routing and File Resolution]
    D --> E[Policy Layer]
    E --> F[Response Builder]
    F --> G[Socket Write]
```

What you get:
- Clear separation between parsing, policy, and response generation.
- One request path for correctness and observability.

## Core and Feature Modules

```mermaid
flowchart TD
    Core[Core Modules<br/>server request response error]
    Async[async_server async_runtime]
    Perf[perf_server]
    Proto[http2_server http3_profile]
    Ent[enterprise]
    Scale[distributed_rate_limit tenant_isolation runtime_autotune]
    Opt[optimized batch streaming observability]

    Core --> Async
    Core --> Perf
    Core --> Proto
    Core --> Ent
    Core --> Scale
    Core --> Opt
```

Interpretation:
- Core modules own baseline behavior.
- Feature-gated modules extend capability without forcing runtime cost when disabled.

## High-Performance Path

```mermaid
flowchart LR
    A[Tokio Listener] --> B[Adaptive Inflight Semaphore]
    B --> C[Queue Guard]
    C --> D[Request Parse]
    D --> E{Static File Fast Path}
    E -->|Yes| F[sendfile path and precompressed negotiation]
    E -->|No| G[Standard response pipeline]
    F --> H[Write Response]
    G --> H
```

Operational effect:
- Backpressure limits protect latency under load.
- Fast-path serving avoids expensive work for common static requests.

## Enterprise Policy Layer

```mermaid
flowchart TD
    Req[Incoming Request] --> Auth[Auth Policy<br/>API key JWT mTLS subject]
    Auth --> AZ[Authorization Hook<br/>RBAC ABAC]
    AZ --> Audit[Structured Access Audit Event]
    Audit --> Obs[Telemetry and Trace Correlation]
```

Design goal:
- Enforce policy at the edge of request handling.
- Produce auditable events with consistent request context.

## Portability and Release Pipeline

```mermaid
flowchart LR
    Dev[Local Dev] --> CI[CI Matrix]
    CI --> T1[Tier 1 Targets]
    CI --> T2[Tier 2 Checks]
    T1 --> Artifacts[Release Artifacts]
    T2 --> Artifacts
    Artifacts --> GH[GitHub Releases]
```

Delivery goal:
- Validate behavior on macOS, Linux, and Windows targets.
- Produce reproducible binaries and container artifacts.
