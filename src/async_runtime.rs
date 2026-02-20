//! Async runtime helpers.

use crate::error::ServerError;

/// Runs a blocking function on Tokio's blocking pool and maps panics/joins to `TaskFailed`.
#[cfg(feature = "async")]
pub async fn run_blocking<F, T>(operation: F) -> Result<T, ServerError>
where
    F: FnOnce() -> Result<T, ServerError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(operation).await {
        Ok(result) => result,
        Err(err) => Err(ServerError::TaskFailed(format!(
            "blocking task failed: {err}"
        ))),
    }
}

/// Non-async fallback for builds without async feature.
#[cfg(not(feature = "async"))]
pub fn run_blocking<F, T>(operation: F) -> Result<T, ServerError>
where
    F: FnOnce() -> Result<T, ServerError>,
{
    operation()
}

#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_blocking_maps_panic_to_task_failed() {
        let result = run_blocking(|| -> Result<(), ServerError> {
            panic!("boom")
        })
        .await;
        assert!(matches!(result, Err(ServerError::TaskFailed(_))));
    }

    #[tokio::test]
    async fn run_blocking_returns_inner_error() {
        let result = run_blocking(|| -> Result<(), ServerError> {
            Err(ServerError::Custom("inner".to_string()))
        })
        .await;
        assert!(matches!(result, Err(ServerError::Custom(_))));
    }

    #[tokio::test]
    async fn run_blocking_returns_success_value() {
        let result =
            run_blocking(|| -> Result<usize, ServerError> { Ok(7) })
                .await
                .expect("ok");
        assert_eq!(result, 7);
    }
}
