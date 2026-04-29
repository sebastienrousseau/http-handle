# Distributed Rate Limiting

`http-handle` now includes distributed rate-limiting adapters in
`src/distributed_rate_limit.rs`.

## Backends

- `InMemoryBackend` for local fallback.
- `RedisBackend<C>` for Redis-like clients.
- `MemcachedBackend<C>` for Memcached-like clients.

## Core Type

- `DistributedRateLimiter<B>` performs source-key checks against any backend
  implementing `RateLimitBackend`.

## Example

See: `examples/ratelimit.rs` (run via `./scripts/example.sh ratelimit`).
