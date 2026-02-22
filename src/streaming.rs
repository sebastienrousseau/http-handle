// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Pull-based chunked streaming utilities for large files.

use crate::error::ServerError;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// A pull-based chunk stream for file content.
///
/// # Examples
///
/// ```rust,no_run
/// use http_handle::streaming::ChunkStream;
/// use std::path::Path;
/// let _stream = ChunkStream::from_file(Path::new("README.md"), 1024);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Debug)]
pub struct ChunkStream {
    reader: BufReader<File>,
    chunk_size: usize,
    exhausted: bool,
}

impl ChunkStream {
    /// Opens a file and returns a chunk stream.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::streaming::ChunkStream;
    /// use std::path::Path;
    /// let stream = ChunkStream::from_file(Path::new("README.md"), 512);
    /// assert!(stream.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when the target file cannot be opened.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn from_file(
        path: &Path,
        chunk_size: usize,
    ) -> Result<Self, ServerError> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            chunk_size: chunk_size.max(1),
            exhausted: false,
        })
    }
}

impl Iterator for ChunkStream {
    type Item = Result<Vec<u8>, ServerError>;

    fn next(&mut self) -> Option<Self::Item> {
        read_next_chunk(
            &mut self.reader,
            self.chunk_size,
            &mut self.exhausted,
        )
    }
}

fn read_next_chunk<R: Read>(
    reader: &mut R,
    chunk_size: usize,
    exhausted: &mut bool,
) -> Option<Result<Vec<u8>, ServerError>> {
    if *exhausted {
        return None;
    }

    let mut buf = vec![0_u8; chunk_size];
    match reader.read(&mut buf) {
        Ok(0) => {
            *exhausted = true;
            None
        }
        Ok(n) => {
            buf.truncate(n);
            Some(Ok(buf))
        }
        Err(err) => {
            *exhausted = true;
            Some(Err(ServerError::Io(err)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tempfile::TempDir;

    struct ErrReader;

    impl Read for ErrReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("boom"))
        }
    }

    #[test]
    fn helper_maps_read_errors_and_marks_exhausted() {
        let mut exhausted = false;
        let mut reader = ErrReader;
        let item = read_next_chunk(&mut reader, 8, &mut exhausted);
        assert!(matches!(item, Some(Err(ServerError::Io(_)))));
        assert!(exhausted);
    }

    #[test]
    fn helper_returns_none_when_already_exhausted() {
        let mut exhausted = true;
        let mut reader = io::empty();
        assert!(
            read_next_chunk(&mut reader, 4, &mut exhausted).is_none()
        );
    }

    #[test]
    fn streams_file_in_chunks() {
        let tmp = TempDir::new().expect("tmp");
        let file = tmp.path().join("data.txt");
        std::fs::write(&file, b"abcdefgh").expect("write");

        let chunks: Result<Vec<Vec<u8>>, _> =
            ChunkStream::from_file(&file, 3).expect("open").collect();

        assert_eq!(
            chunks.expect("chunks"),
            vec![b"abc".to_vec(), b"def".to_vec(), b"gh".to_vec()]
        );
    }

    #[test]
    fn missing_file_returns_io_error() {
        let tmp = TempDir::new().expect("tmp");
        let missing = tmp.path().join("does-not-exist.txt");
        let result = ChunkStream::from_file(&missing, 4);
        assert!(matches!(result, Err(ServerError::Io(_))));
    }

    #[test]
    fn returns_none_after_stream_is_exhausted() {
        let tmp = TempDir::new().expect("tmp");
        let file = tmp.path().join("single-byte.txt");
        std::fs::write(&file, b"x").expect("write");
        let mut stream =
            ChunkStream::from_file(&file, 1).expect("stream open");

        assert!(
            matches!(stream.next(), Some(Ok(chunk)) if chunk == b"x")
        );
        assert!(stream.next().is_none());
        assert!(stream.next().is_none());
    }
}
