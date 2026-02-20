//! Batch processing utilities for concurrent file operations.

use crate::error::ServerError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

/// Batch operation request.
#[derive(Clone, Debug)]
pub struct BatchRequest {
    /// Relative path to read.
    pub relative_path: PathBuf,
}

/// Batch operation result.
#[derive(Debug)]
pub struct BatchResult {
    /// Requested relative path.
    pub relative_path: PathBuf,
    /// Read bytes if successful.
    pub body: Result<Vec<u8>, ServerError>,
}

/// Concurrently reads multiple files under a shared root.
pub fn process_batch(
    document_root: &Path,
    requests: &[BatchRequest],
    workers: usize,
) -> Vec<BatchResult> {
    if requests.is_empty() {
        return Vec::new();
    }

    let workers = workers.max(1).min(requests.len());
    let shared =
        Arc::new(Mutex::new(Vec::with_capacity(requests.len())));

    thread::scope(|scope| {
        let chunk_size = requests.len().div_ceil(workers);
        for chunk in requests.chunks(chunk_size) {
            let root = document_root.to_path_buf();
            let out = Arc::clone(&shared);
            let _ = scope.spawn(move || {
                for req in chunk {
                    let full_path = root.join(&req.relative_path);
                    let result =
                        fs::read(&full_path).map_err(ServerError::from);
                    let entry = BatchResult {
                        relative_path: req.relative_path.clone(),
                        body: result,
                    };
                    if let Ok(mut guard) = out.lock() {
                        guard.push(entry);
                    }
                }
            });
        }
    });

    let mut guard = shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut out = std::mem::take(&mut *guard);
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn batch_reads_multiple_files() {
        let tmp = TempDir::new().expect("tmp");
        fs::write(tmp.path().join("a.txt"), b"a").expect("write a");
        fs::write(tmp.path().join("b.txt"), b"b").expect("write b");

        let requests = vec![
            BatchRequest {
                relative_path: PathBuf::from("a.txt"),
            },
            BatchRequest {
                relative_path: PathBuf::from("b.txt"),
            },
        ];

        let results = process_batch(tmp.path(), &requests, 2);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.body.is_ok()));
    }

    #[test]
    fn batch_returns_empty_for_empty_requests() {
        let tmp = TempDir::new().expect("tmp");
        let requests: Vec<BatchRequest> = Vec::new();
        let results = process_batch(tmp.path(), &requests, 4);
        assert!(results.is_empty());
    }

    #[test]
    fn batch_reports_missing_file_error() {
        let tmp = TempDir::new().expect("tmp");
        let requests = vec![BatchRequest {
            relative_path: PathBuf::from("missing.txt"),
        }];

        let results = process_batch(tmp.path(), &requests, 1);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].body, Err(ServerError::Io(_))));
    }
}
