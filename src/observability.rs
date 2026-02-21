//! Observability initialization helpers.

#[cfg(feature = "observability")]
use tracing_subscriber::EnvFilter;

/// Initializes default tracing subscriber once.
///
/// This is a no-op when the `observability` feature is disabled.
///
/// # Examples
///
/// ```rust
/// use http_handle::observability::init_tracing;
/// init_tracing();
/// assert_eq!(1 + 1, 2);
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn init_tracing() {
    #[cfg(feature = "observability")]
    {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .try_init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_tracing_is_safe_to_call() {
        init_tracing();
        init_tracing();
    }
}
