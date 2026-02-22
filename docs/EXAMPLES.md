# Examples Coverage Matrix

This document maps `http-handle` capabilities to runnable examples so
functional coverage stays explicit across code, docs, and CI.

## Run Any Example

```shell
cargo run --example <name> [--features "..."]
```

## Core Runtime

| Capability | Example |
|---|---|
| Basic static server | `basic_server` |
| Full server setup flow | `server_example` |
| Builder configuration | `server_builder_example` |
| Graceful shutdown | `graceful_shutdown_example` |
| End-to-end walkthrough | `full_demo` |
| Legacy compatibility flow | `lib_example_legacy` |

## HTTP Primitives

| Capability | Example |
|---|---|
| Request parsing | `request_example` |
| Response construction | `response_example` |
| Error handling | `error_example` |

## Performance and Operations

| Capability | Example |
|---|---|
| Thread/connection pooling behavior | `pooling_performance_example` |
| Server policy scenario | `scenario_server_policies` |
| External benchmark target | `benchmark_target` |

## Feature-Gated Modules

| Capability | Feature | Example |
|---|---|---|
| Async runtime helper | `async` | `feature_async_runtime` |
| Async server path | `async` | `feature_async_server` |
| Runtime language detection | `default` | `feature_language_detection` |
| Batch processing | `batch` | `feature_batch_processing` |
| Streaming chunks | `streaming` | `feature_streaming_chunks` |
| Optimized lookups | `optimized` | `feature_optimized_lookups` |
| Observability setup | `observability` | `feature_observability` |
| HTTP/2 server path | `http2` | `feature_http2_server` |
| HTTP/3 profile policy | `http3-profile` | `feature_http3_profile` |
| Enterprise authorization | `enterprise` | `feature_enterprise_authorization` |
| Distributed rate limiting | `distributed-rate-limit` | `feature_distributed_rate_limit` |
| Multi-tenant isolation | `multi-tenant` | `feature_tenant_isolation` |
| Runtime auto-tune profile | `autotune` | `feature_runtime_autotune` |

## Validation Commands

```shell
# compile all examples with all optional features enabled
cargo check --all-features --examples

# run all tests and doctests
cargo test --all-features
```
