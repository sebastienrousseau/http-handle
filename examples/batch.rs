// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `batch` feature: concurrent file reads under a fixed parallelism cap.
//!
//! Run: `cargo run --example batch --features batch`

#[cfg(feature = "batch")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "batch")]
fn main() {
    use http_handle::batch::{BatchRequest, process_batch};
    use std::path::PathBuf;

    support::header("http-handle -- batch");

    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("a.txt"), b"alpha").expect("a");
    std::fs::write(root.path().join("b.txt"), b"beta").expect("b");
    std::fs::write(root.path().join("c.txt"), b"gamma").expect("c");

    support::task_with_output(
        "Read 3 files at parallelism=2, success path",
        || {
            let requests = ["a.txt", "b.txt", "c.txt"]
                .into_iter()
                .map(|name| BatchRequest {
                    relative_path: PathBuf::from(name),
                })
                .collect::<Vec<_>>();

            process_batch(root.path(), &requests, 2)
                .into_iter()
                .map(|result| match result.body {
                    Ok(bytes) => format!(
                        "{} ok ({} bytes)",
                        result.relative_path.display(),
                        bytes.len()
                    ),
                    Err(error) => format!(
                        "{} err: {error}",
                        result.relative_path.display()
                    ),
                })
                .collect()
        },
    );

    support::task_with_output(
        "Missing file surfaces a per-request error without aborting the batch",
        || {
            let requests = vec![
                BatchRequest {
                    relative_path: PathBuf::from("a.txt"),
                },
                BatchRequest {
                    relative_path: PathBuf::from("does-not-exist.txt"),
                },
            ];
            process_batch(root.path(), &requests, 2)
                .into_iter()
                .map(|result| {
                    let path = result.relative_path.display();
                    match result.body {
                        Ok(_) => format!("{path} ok"),
                        Err(error) => format!("{path} err: {error}"),
                    }
                })
                .collect()
        },
    );

    support::summary(2);
}

#[cfg(not(feature = "batch"))]
fn main() {
    eprintln!(
        "Enable the 'batch' feature: cargo run --example batch --features batch"
    );
}
