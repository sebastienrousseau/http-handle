// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Concurrent batch file processing example.

#[cfg(feature = "batch")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::batch::{BatchRequest, process_batch};
    use std::path::PathBuf;

    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.txt"), b"alpha")?;
    std::fs::write(root.path().join("b.txt"), b"beta")?;

    let requests = vec![
        BatchRequest {
            relative_path: PathBuf::from("a.txt"),
        },
        BatchRequest {
            relative_path: PathBuf::from("b.txt"),
        },
    ];

    let results = process_batch(root.path(), &requests, 2);
    for result in results {
        match result.body {
            Ok(bytes) => println!(
                "{} => {} bytes",
                result.relative_path.display(),
                bytes.len()
            ),
            Err(error) => {
                println!(
                    "{} => error: {error}",
                    result.relative_path.display()
                )
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "batch"))]
fn main() {
    eprintln!(
        "Enable the 'batch' feature: cargo run --example feature_batch_processing --features batch"
    );
}
