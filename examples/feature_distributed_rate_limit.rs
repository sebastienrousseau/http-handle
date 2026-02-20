//! Example showing distributed rate limiting with in-memory fallback backend.

#[cfg(feature = "distributed-rate-limit")]
use http_handle::distributed_rate_limit::{
    DistributedRateLimiter, InMemoryBackend,
};

fn main() {
    #[cfg(feature = "distributed-rate-limit")]
    {
        let limiter = DistributedRateLimiter::new(
            InMemoryBackend::default(),
            "example",
            2,
            60,
        );
        let source = "127.0.0.1".parse().expect("ip");
        let first = limiter.is_limited(source).expect("check");
        let second = limiter.is_limited(source).expect("check");
        let third = limiter.is_limited(source).expect("check");
        println!(
            "limited states -> first={first} second={second} third={third}"
        );
    }
}
