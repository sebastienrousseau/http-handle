// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `streaming` feature: pull-based `ChunkStream` for large files.
//!
//! Run: `cargo run --example streaming --features streaming`

#[cfg(feature = "streaming")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "streaming")]
fn main() {
    use http_handle::streaming::ChunkStream;

    support::header("http-handle -- streaming");

    let root = tempfile::tempdir().expect("tempdir");
    let file = root.path().join("data.txt");
    std::fs::write(&file, b"abcdefghijk").expect("seed");

    support::task_with_output(
        "Iterate an 11-byte file in 4-byte chunks",
        || {
            let stream =
                ChunkStream::from_file(&file, 4).expect("open");
            stream
                .enumerate()
                .map(|(i, chunk)| {
                    let bytes = chunk.expect("chunk");
                    format!(
                        "chunk[{i}] = {:?}",
                        String::from_utf8_lossy(&bytes)
                    )
                })
                .collect()
        },
    );

    support::task_with_output(
        "ChunkStream is exhausted after the final partial chunk",
        || {
            let mut stream =
                ChunkStream::from_file(&file, 4).expect("open");
            let mut yielded = 0;
            while stream.next().is_some() {
                yielded += 1;
            }
            vec![
                format!("yielded     = {yielded} chunks"),
                format!(
                    "next() now  = {:?}",
                    stream.next().map(|_| "Some(_)").unwrap_or("None")
                ),
            ]
        },
    );

    support::summary(2);
}

#[cfg(not(feature = "streaming"))]
fn main() {
    eprintln!(
        "Enable the 'streaming' feature: cargo run --example streaming --features streaming"
    );
}
