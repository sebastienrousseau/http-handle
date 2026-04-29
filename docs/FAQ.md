# Frequently Asked Questions

These are the questions that come up most often when teams adopt
`http-handle`. Short answers in the README; the long-form is here.

## Choosing a server mode

### Q. Which `start*` should I use?

The crate ships five entry points; pick by deployment shape, not by
"which is fastest". See `docs/PERFORMANCE.md` for measured numbers.

| Entry point | When | Trade-off |
|---|---|---|
| `Server::start()` | Default, simplest, no async dep. | Spawns one OS thread per accepted connection. Fine up to thousands of concurrent connections on a memory-rich host. |
| `Server::start_with_thread_pool(n)` | Bounded worker count for predictable resource use. | New connections block briefly when all workers are busy. |
| `Server::start_with_pooling(workers, max_connections)` | Production deployments where you also want to cap concurrent connections. | Same as above plus a hard 503 when the connection pool is full. |
| `Server::start_with_graceful_shutdown(timeout)` | SIGINT / SIGTERM-aware exit. | Wraps `start()` with a drain loop that waits up to `timeout` for in-flight connections. |
| `perf_server::start_high_perf(server, limits)` (`feature = "high-perf"`) | Memory-constrained hosts, mixed I/O workloads, or one-process-per-CPU deployments behind a load balancer. | Single OS thread, async accept loop, semaphore-bounded inflight + queue, sendfile fast-path on Linux. |
| `perf_server::start_high_perf_multi_thread(server, limits, workers)` (`feature = "high-perf-multi-thread"`) | Same workloads as `start_high_perf` but on multi-core hosts where work-stealing helps. As of v0.0.5 this is the **throughput leader** on every Linux row in `docs/PERFORMANCE.md`. | Builds a multi-thread Tokio runtime internally — callers don't need `rt-multi-thread` on their own tokio dep. |

### Q. When does sync win, and when does async win?

In v0.0.5, on a CPU-bound static-file workload (no I/O wait per
request), `high-perf-mt` overtakes sync once the response cache and
coalesced write are in play. The crossover happens because the cache
removes the per-request `format!` + `std::fs::read` cost, leaving
multi-thread async with only the cost it's good at: spreading work
across cores.

For workloads with **real I/O wait** (database calls, slow upstream
services, multiplexed sockets), `high-perf-mt` pulls further ahead —
the async runtime can park a future during the wait while sync would
block an OS thread.

For pure static-file serving on a memory-rich host, sync's
thread-per-connection model is still competitive because the OS
scheduler already spreads work across cores and there's no async
overhead to amortise.

### Q. Do I need the `high-perf-multi-thread` feature?

Only if you call `start_high_perf_multi_thread`. Enabling the feature
pulls in `tokio/rt-multi-thread`, which is a non-trivial code-size
addition. Use `high-perf` (single-threaded async) for memory- or
binary-size-constrained deployments.

## Performance and benchmarks

### Q. Are the numbers in `docs/PERFORMANCE.md` realistic for my workload?

They're upper bounds for a small static body over loopback. The Linux
row (180 k req/s on `high-perf-mt`) is more representative for
production than the macOS row, because it tests the real `epoll`
networking path. Numbers will differ in absolute terms on x86_64 vs
arm64 hardware; the relative ordering between modes should hold.

For your workload, run `scripts/load_test.sh <mode> <duration> <conns>`
locally — it builds the `bench` example with the right features and
drives it via bombardier.

### Q. How does the response cache work? Can I disable it?

The high-perf static-file fast path caches a pre-serialised
`(head_prefix, body)` keyed by `(canonical_path, mtime_secs, file_len)`.
On hit, the per-request cost is a tiny `Connection:` header format,
one buffer concatenation, and one `write_all` syscall. The cache cap is
`RESPONSE_CACHE_MAX = 256` entries, body-size-gated by
`sendfile_threshold_bytes` (default 64 KiB), so worst-case memory is
~16 MiB.

Above-threshold files (large media, downloads) skip the cache entirely
and use sendfile (Linux) or `tokio::io::copy` (other platforms). There
is no way to disable the cache today — open an issue if you have a
workload where it would be a regression and we'll add a feature flag.

Cache invalidation is automatic: a touch / replace of the file changes
`mtime_secs`, which makes the old key un-findable. The new request
inserts a fresh entry; the stale entry stays until the cap forces an
eviction.

### Q. The benchmark shows huge numbers but my real server is slower. Why?

Loopback elides almost every real-network cost: TCP slow-start, packet
loss, NIC offload, asymmetric latency, contention from the load tool
itself. Treat the bench as an **upper bound**, not a forecast. Real
deployments behind a load balancer with TLS termination will see lower
absolute throughput; the relative ordering between modes still holds.

## Security

### Q. Does `http-handle` enforce any security policies by default?

The synchronous server applies a small built-in set: directory
traversal rejection (paths containing `..` or escaping the canonical
document root), `Content-Type` sniffing-friendly headers when the file
extension is recognised, and a 64 MiB cap on buffered response bodies
(closes the OOM vector where a 1 GB file would drive RSS to N ×
file_size on N concurrent requests).

Everything else (CORS, custom security headers, rate limit, TLS, mTLS,
auth) is opt-in via `ServerBuilder` or the `enterprise` umbrella
feature.

### Q. Can I run `http-handle` over HTTPS?

The crate's TLS primitives live in `enterprise::TlsPolicy`
(`feature = "enterprise"`) — they describe configuration intent (cert
chain path, private key path, mTLS client CA bundle, mTLS subject
allowlist) but **don't terminate TLS in-process**. The recommended
deployment is one of:

1. Terminate TLS at a reverse proxy (nginx, Caddy, HAProxy, AWS ALB) and
   speak plaintext HTTP/1.1 or h2c to `http-handle` over loopback.
2. Front the server with `cloudflared` / `ngrok` / similar.

In-process TLS termination via `rustls` is on the v0.1 roadmap.

### Q. What about HTTP/3?

`feature = "http3-profile"` ships ALPN routing and a fallback chain
(`Http3 → Http2 → Http1`) so a load balancer can negotiate the right
protocol. The crate does **not** terminate QUIC or speak h3 frames in
v0.0.5 — that's deferred to v0.2 with a design proposal in
`docs/HTTP3_DESIGN.md` (quinn + h3 + h3-quinn).

### Q. The rate limiter — is it per-instance or distributed?

Two separate primitives:

- **In-process rate limiter** (default, no feature flag): sharded
  `[Mutex<HashMap<IpAddr, Vec<Instant>>>; 16]` keyed by hash of the
  client IP. 16-way sharding cuts effective contention 16× compared to a
  single global mutex. Configure via `ServerBuilder::rate_limit_per_minute(n)`.
- **Distributed rate limiter** (`feature = "distributed-rate-limit"`):
  `DistributedRateLimiter<B>` where `B: RateLimitBackend`. Ships an
  `InMemoryBackend` for testing and a trait you can implement against
  Redis / DynamoDB / memcached. See `docs/DISTRIBUTED_RATE_LIMITING.md`.

## Deployment

### Q. How do I deploy this behind nginx / a reverse proxy?

Front nginx terminates TLS, sets X-Forwarded-* headers, and proxies to
`http-handle` on loopback. Example nginx config skeleton:

```nginx
upstream http_handle {
    server 127.0.0.1:8080;
    keepalive 64;
}

server {
    listen 443 ssl http2;
    ssl_certificate /etc/ssl/full.crt;
    ssl_certificate_key /etc/ssl/server.key;

    location / {
        proxy_pass http://http_handle;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

`proxy_http_version 1.1` + `Connection: ""` enables nginx to reuse a
keep-alive pool with `http-handle` (which speaks HTTP/1.1 keep-alive
since v0.0.5). For multi-tenant deployments use `multi-tenant` feature
to isolate config and secrets per `TenantId`.

### Q. How do I deploy in Kubernetes / Docker?

`http-handle` produces a single binary with no runtime dependencies
beyond the system libc. A minimal container build:

```dockerfile
FROM rust:1.88-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release --features 'high-perf-multi-thread'

FROM gcr.io/distroless/cc-debian12
COPY --from=build /src/target/release/http-handle /http-handle
EXPOSE 8080
ENTRYPOINT ["/http-handle"]
```

Bind to `0.0.0.0` (the default when `HTTP_HANDLE_ADDR` is unset) and
expose via a Service. Wire `start_with_graceful_shutdown(timeout)` to
the SIGTERM Kubernetes sends on pod termination so in-flight
connections drain.

### Q. How do I tune thread count / connection cap for production?

Two knobs that matter:

- **`start_with_pooling(workers, max_connections)`** — workers should
  typically be 2-4× CPU cores; max_connections is bounded by available
  memory and file descriptors. Default ulimit on most distros caps you
  at 1024 fds per process; raise via systemd `LimitNOFILE` or your
  orchestrator's equivalent.
- **`start_high_perf_multi_thread(server, limits, worker_threads)`** —
  `worker_threads = None` defaults to logical CPU count. Pass
  `Some(n)` to pin (useful for reproducible benchmarking or when CPU
  affinity is set externally). `PerfLimits::max_inflight` defaults to
  256 and bounds concurrent in-flight requests; `max_queue` (default
  1024) is a soft queue past `max_inflight`.

`feature = "autotune"` derives all of this from the detected host
profile (`HostResourceProfile { cpu_cores, memory_mib }`), so for
container deployments you can do:

```rust,ignore
use http_handle::runtime_autotune::{detect_host_profile, RuntimeTuneRecommendation};

let limits = RuntimeTuneRecommendation::from_profile(detect_host_profile())
    .into_perf_limits();
http_handle::perf_server::start_high_perf_multi_thread(server, limits, None)?;
```

## Integration

### Q. Can I plug in my own request handler?

Not yet — `http-handle` is currently a **static-file server** with
configurable policies (CORS, headers, rate limit, cache TTL, etc.). A
request-handler trait is on the v0.1 roadmap. For dynamic routes
today, run `http-handle` for `/static/*` and a dynamic framework
(`axum`, `actix-web`) for the rest, joined by a path prefix in nginx.

### Q. Does it support gzip / brotli / zstd?

Yes, **statically-precompressed assets** are served when the client
sends `Accept-Encoding: br | zstd | gzip`. The server looks for
sibling files with `.br` / `.zst` / `.gz` next to the requested
path. The selection order is `br` → `zstd` → `gzip`, matching what
nginx's `ngx_http_gzip_static_module` does.

There's no runtime compression today (would add CPU cost and a
streaming-body API the crate doesn't have yet). Pre-compress at build
time:

```bash
brotli -k -q 11 public/index.html       # produces index.html.br
zstd  -k -19  public/index.html         # produces index.html.zst
gzip  -k -9   public/index.html         # produces index.html.gz
```

### Q. How do I observe what the server is doing?

`feature = "observability"` wires `tracing` + `tracing-subscriber`.
Initialise via `http_handle::observability::init_tracing()` and the
crate emits structured events under the `http_handle::*` target.

For metrics / OTLP export, the `enterprise::TelemetryPolicy` struct
holds the OTLP endpoint config; the actual exporter is left to your
chosen provider (the crate doesn't bundle an OTLP HTTP client).

## Errors and troubleshooting

### Q. I'm getting `error: target 'enterprise' requires the features: 'enterprise'`. What do I do?

Use the wrapper:

```bash
./scripts/example.sh enterprise
```

Or pass the flag explicitly:

```bash
cargo run --features enterprise --example enterprise
```

The README's `## Examples` table shows the literal copy-paste command
for every gated example. `./scripts/example.sh --list` prints all 30
example names.

### Q. My CI fails with `Invalid enum value. Expected 'critical' | 'high' | 'moderate' | 'low', received 'medium'`.

That's an upstream `actions/dependency-review-action` defaults issue
in `pipelines@v0.0.2`'s `security.yml`. The current `ci.yml` overrides
`severity-threshold: moderate` to route around it. If you've forked
the workflow, mirror that override.

### Q. The tests pass locally but fail in CI with a port collision.

Tests bind to `127.0.0.1:0` and use the kernel-assigned port. Some CI
runners have aggressive cleanup that doesn't release the port between
test invocations. The bombardier harness in `scripts/load_test.sh`
uses Python's `socket` module to probe-bind and then drop the
listener; reproduce that pattern in custom test scaffolding if you
hit the same collision.

### Q. `cargo build --all-features` fails on Windows with sendfile-related errors.

`feature = "high-perf"` is Unix-only because `libc::sendfile` is the
fast-path on Linux (with a no-op stub on macOS). The crate compiles
clean on Windows under the default and `async` / `http2` /
`http3-profile` / `enterprise` feature sets — just don't enable
`high-perf*`.

## Versioning and stability

### Q. What's the API stability guarantee on 0.0.x?

The crate follows pre-1.0 SemVer: any 0.0.x release line may include
breaking changes, but every breaking change is documented in
`CHANGELOG.md` with a migration note in `docs/MIGRATION_GUIDE.md`.
v0.0.5 made four breaking changes; the migration guide includes
copy-paste-ready before/after snippets.

For production use, pin to an exact version (`http-handle = "=0.0.5"`)
until v0.1.

### Q. When will v0.1 ship?

When in-process TLS termination, request-handler traits, and the
streaming response API land. No firm date — track progress via the
HTTP/3 design doc (`docs/HTTP3_DESIGN.md`) and the v0.1 issues
tagged `roadmap/v0.1` on GitHub.

## Contributing

See `CONTRIBUTING.md` for the dev workflow, signed commits, and PR
guidelines. The short version:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

All three must be clean before opening a PR. New behaviour ships with
new tests in the same commit.
