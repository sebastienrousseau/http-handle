// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Chunked streaming example for large files.

#[cfg(feature = "streaming")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use http_handle::streaming::ChunkStream;

    let root = tempfile::tempdir()?;
    let file = root.path().join("data.txt");
    std::fs::write(&file, b"abcdefghijk")?;

    let stream = ChunkStream::from_file(&file, 4)?;
    for (index, chunk) in stream.enumerate() {
        let bytes = chunk?;
        println!(
            "chunk[{index}] = {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }

    Ok(())
}

#[cfg(not(feature = "streaming"))]
fn main() {
    eprintln!(
        "Enable the 'streaming' feature: cargo run --example feature_streaming_chunks --features streaming"
    );
}
