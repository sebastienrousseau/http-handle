// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Distributed rate-limiting adapters and backend contracts.

use crate::error::ServerError;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Backend trait for incrementing a rate-limit key in a time window.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::RateLimitBackend;
/// # let _ = std::any::TypeId::of::<&dyn RateLimitBackend>();
/// assert_eq!(2 + 2, 4);
/// ```
///
/// # Panics
///
/// Trait usage does not panic by itself.
pub trait RateLimitBackend: Send + Sync + std::fmt::Debug {
    /// Increments key and returns current hit count for the active window.
    fn increment_and_get(
        &self,
        key: &str,
        window_secs: u64,
    ) -> Result<u64, ServerError>;
}

/// Shared rate limiter that works against pluggable backends.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::{DistributedRateLimiter, InMemoryBackend};
/// let _limiter = DistributedRateLimiter::new(InMemoryBackend::default(), "ip", 100, 60);
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Debug)]
pub struct DistributedRateLimiter<B: RateLimitBackend> {
    backend: Arc<B>,
    namespace: String,
    limit_per_window: u64,
    window_secs: u64,
}

impl<B: RateLimitBackend> DistributedRateLimiter<B> {
    /// Creates a distributed limiter with explicit namespace and limits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::distributed_rate_limit::{DistributedRateLimiter, InMemoryBackend};
    /// let _ = DistributedRateLimiter::new(InMemoryBackend::default(), "ip", 10, 60);
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn new(
        backend: B,
        namespace: impl Into<String>,
        limit_per_window: u64,
        window_secs: u64,
    ) -> Self {
        Self {
            backend: Arc::new(backend),
            namespace: namespace.into(),
            limit_per_window: limit_per_window.max(1),
            window_secs: window_secs.max(1),
        }
    }

    /// Returns true when the source should be throttled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::distributed_rate_limit::{DistributedRateLimiter, InMemoryBackend};
    /// use std::net::IpAddr;
    /// let limiter = DistributedRateLimiter::new(InMemoryBackend::default(), "ip", 1, 60);
    /// let ip: IpAddr = "127.0.0.1".parse().expect("ip");
    /// let _ = limiter.is_limited(ip);
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns backend errors when increment operations fail.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn is_limited(
        &self,
        source: IpAddr,
    ) -> Result<bool, ServerError> {
        let key = format!("{}:{}", self.namespace, source);
        let current =
            self.backend.increment_and_get(&key, self.window_secs)?;
        Ok(current > self.limit_per_window)
    }
}

/// In-memory backend useful for local fallback mode and tests.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::InMemoryBackend;
/// let _backend = InMemoryBackend::default();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Debug, Default)]
pub struct InMemoryBackend {
    state: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimitBackend for InMemoryBackend {
    fn increment_and_get(
        &self,
        key: &str,
        window_secs: u64,
    ) -> Result<u64, ServerError> {
        let now = Instant::now();
        let mut state = self.state.lock().map_err(|_| {
            ServerError::Custom("rate state poisoned".into())
        })?;
        let hits = state.entry(key.to_string()).or_default();
        hits.retain(|ts| {
            now.duration_since(*ts) <= Duration::from_secs(window_secs)
        });
        hits.push(now);
        Ok(hits.len() as u64)
    }
}

/// Minimal Redis-like client contract.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::RedisClient;
/// # let _ = std::any::TypeId::of::<&dyn RedisClient>();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// Trait usage does not panic by itself.
pub trait RedisClient: Send + Sync + std::fmt::Debug {
    /// Increments key, sets TTL as needed, and returns current count.
    fn incr_with_ttl(
        &self,
        key: &str,
        ttl_secs: u64,
    ) -> Result<u64, ServerError>;
}

/// Redis backend adapter.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::{RedisBackend, RedisClient};
/// use http_handle::ServerError;
/// #[derive(Debug)]
/// struct Dummy;
/// impl RedisClient for Dummy {
///     fn incr_with_ttl(&self, _key: &str, _ttl_secs: u64) -> Result<u64, ServerError> { Ok(1) }
/// }
/// let _backend = RedisBackend::new(Dummy);
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Debug)]
pub struct RedisBackend<C: RedisClient> {
    client: C,
}

impl<C: RedisClient> RedisBackend<C> {
    /// Creates a new Redis backend adapter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::distributed_rate_limit::{RedisBackend, RedisClient};
    /// use http_handle::ServerError;
    /// #[derive(Debug)]
    /// struct Dummy;
    /// impl RedisClient for Dummy {
    ///     fn incr_with_ttl(&self, _key: &str, _ttl_secs: u64) -> Result<u64, ServerError> { Ok(1) }
    /// }
    /// let _backend = RedisBackend::new(Dummy);
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn new(client: C) -> Self {
        Self { client }
    }
}

impl<C: RedisClient> RateLimitBackend for RedisBackend<C> {
    fn increment_and_get(
        &self,
        key: &str,
        window_secs: u64,
    ) -> Result<u64, ServerError> {
        self.client.incr_with_ttl(key, window_secs)
    }
}

/// Minimal Memcached-like client contract.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::MemcachedClient;
/// # let _ = std::any::TypeId::of::<&dyn MemcachedClient>();
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// Trait usage does not panic by itself.
pub trait MemcachedClient: Send + Sync + std::fmt::Debug {
    /// Increments key and returns current count.
    fn incr(
        &self,
        key: &str,
        initial: u64,
        ttl_secs: u32,
    ) -> Result<u64, ServerError>;
}

/// Memcached backend adapter.
///
/// # Examples
///
/// ```rust
/// use http_handle::distributed_rate_limit::{MemcachedBackend, MemcachedClient};
/// use http_handle::ServerError;
/// #[derive(Debug)]
/// struct Dummy;
/// impl MemcachedClient for Dummy {
///     fn incr(&self, _key: &str, _initial: u64, _ttl_secs: u32) -> Result<u64, ServerError> { Ok(1) }
/// }
/// let _backend = MemcachedBackend::new(Dummy);
/// assert_eq!(1, 1);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Debug)]
pub struct MemcachedBackend<C: MemcachedClient> {
    client: C,
}

impl<C: MemcachedClient> MemcachedBackend<C> {
    /// Creates a new Memcached backend adapter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::distributed_rate_limit::{MemcachedBackend, MemcachedClient};
    /// use http_handle::ServerError;
    /// #[derive(Debug)]
    /// struct Dummy;
    /// impl MemcachedClient for Dummy {
    ///     fn incr(&self, _key: &str, _initial: u64, _ttl_secs: u32) -> Result<u64, ServerError> { Ok(1) }
    /// }
    /// let _backend = MemcachedBackend::new(Dummy);
    /// assert_eq!(1, 1);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn new(client: C) -> Self {
        Self { client }
    }
}

impl<C: MemcachedClient> RateLimitBackend for MemcachedBackend<C> {
    fn increment_and_get(
        &self,
        key: &str,
        window_secs: u64,
    ) -> Result<u64, ServerError> {
        self.client.incr(key, 1, window_secs as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct MockRedis {
        counts: Mutex<HashMap<String, u64>>,
    }
    impl RedisClient for MockRedis {
        fn incr_with_ttl(
            &self,
            key: &str,
            _ttl_secs: u64,
        ) -> Result<u64, ServerError> {
            let mut counts = self
                .counts
                .lock()
                .map_err(|_| ServerError::Custom("poisoned".into()))?;
            let entry = counts.entry(key.to_string()).or_insert(0);
            *entry += 1;
            Ok(*entry)
        }
    }

    #[derive(Debug, Default)]
    struct MockMemcached {
        counts: Mutex<HashMap<String, u64>>,
    }
    impl MemcachedClient for MockMemcached {
        fn incr(
            &self,
            key: &str,
            initial: u64,
            _ttl_secs: u32,
        ) -> Result<u64, ServerError> {
            let mut counts = self
                .counts
                .lock()
                .map_err(|_| ServerError::Custom("poisoned".into()))?;
            if let Some(entry) = counts.get_mut(key) {
                *entry += 1;
                Ok(*entry)
            } else {
                let _ = counts.insert(key.to_string(), initial);
                Ok(initial)
            }
        }
    }

    #[test]
    fn in_memory_backend_enforces_limit() {
        let limiter = DistributedRateLimiter::new(
            InMemoryBackend::default(),
            "ip",
            2,
            60,
        );
        let ip: IpAddr = "127.0.0.1".parse().expect("ip");
        assert!(!limiter.is_limited(ip).expect("limit"));
        assert!(!limiter.is_limited(ip).expect("limit"));
        assert!(limiter.is_limited(ip).expect("limit"));
    }

    #[test]
    fn redis_adapter_routes_calls() {
        let backend = RedisBackend::new(MockRedis::default());
        let limiter = DistributedRateLimiter::new(backend, "ip", 1, 60);
        let ip: IpAddr = "127.0.0.2".parse().expect("ip");
        assert!(!limiter.is_limited(ip).expect("limit"));
        assert!(limiter.is_limited(ip).expect("limit"));
    }

    #[test]
    fn memcached_adapter_routes_calls() {
        let backend = MemcachedBackend::new(MockMemcached::default());
        let limiter = DistributedRateLimiter::new(backend, "ip", 1, 60);
        let ip: IpAddr = "127.0.0.3".parse().expect("ip");
        assert!(!limiter.is_limited(ip).expect("limit"));
        assert!(limiter.is_limited(ip).expect("limit"));
    }

    #[test]
    fn limiter_propagates_backend_errors() {
        #[derive(Debug)]
        struct FailingBackend;
        impl RateLimitBackend for FailingBackend {
            fn increment_and_get(
                &self,
                _key: &str,
                _window_secs: u64,
            ) -> Result<u64, ServerError> {
                Err(ServerError::Custom("backend down".into()))
            }
        }

        let limiter =
            DistributedRateLimiter::new(FailingBackend, "ip", 0, 0);
        let ip: IpAddr = "127.0.0.9".parse().expect("ip");
        let err = limiter.is_limited(ip).expect_err("must fail");
        assert!(err.to_string().contains("backend down"));
    }

    #[test]
    fn in_memory_backend_maps_poisoned_lock_to_error() {
        let backend = InMemoryBackend::default();
        let _ = std::panic::catch_unwind(|| {
            let _guard = backend.state.lock().expect("lock");
            panic!("poison lock");
        });
        let err = backend
            .increment_and_get("ip:127.0.0.1", 60)
            .expect_err("poisoned lock should error");
        assert!(err.to_string().contains("poisoned"));
    }
}
