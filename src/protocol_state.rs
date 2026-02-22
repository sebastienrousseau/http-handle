// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Protocol byte-classification helpers for fuzzing and conformance tests.

/// Classification outcome for input protocol bytes.
///
/// # Examples
///
/// ```rust
/// use http_handle::protocol_state::ProtocolClassification;
/// assert_eq!(ProtocolClassification::Unknown as u8, ProtocolClassification::Unknown as u8);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolClassification {
    /// Input looks like HTTP/2 client preface.
    Http2Preface,
    /// Input looks like TLS record stream.
    TlsLike,
    /// Input is unclassified or incomplete.
    Unknown,
}

const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// Classifies protocol bytes without panicking.
///
/// # Examples
///
/// ```rust
/// use http_handle::protocol_state::{classify_protocol_bytes, ProtocolClassification};
/// let c = classify_protocol_bytes(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
/// assert_eq!(c, ProtocolClassification::Http2Preface);
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn classify_protocol_bytes(input: &[u8]) -> ProtocolClassification {
    if input.is_empty() {
        return ProtocolClassification::Unknown;
    }
    if H2_PREFACE.starts_with(input) || input.starts_with(H2_PREFACE) {
        return ProtocolClassification::Http2Preface;
    }
    if is_tls_like(input) {
        return ProtocolClassification::TlsLike;
    }
    ProtocolClassification::Unknown
}

fn is_tls_like(input: &[u8]) -> bool {
    if input.len() < 5 {
        return false;
    }
    let content_type = input[0];
    let version_major = input[1];
    matches!(content_type, 20..=23) && version_major == 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_http2_preface() {
        assert_eq!(
            classify_protocol_bytes(H2_PREFACE),
            ProtocolClassification::Http2Preface
        );
    }

    #[test]
    fn classifies_tls_like() {
        let tls = [22_u8, 3, 3, 0, 42, 1, 0, 0, 38];
        assert_eq!(
            classify_protocol_bytes(&tls),
            ProtocolClassification::TlsLike
        );
    }

    #[test]
    fn unknown_for_random_bytes() {
        let data = [1_u8, 2, 3, 4, 5, 6];
        assert_eq!(
            classify_protocol_bytes(&data),
            ProtocolClassification::Unknown
        );
    }

    #[test]
    fn unknown_for_empty_or_short_frames() {
        assert_eq!(
            classify_protocol_bytes(&[]),
            ProtocolClassification::Unknown
        );
        assert_eq!(
            classify_protocol_bytes(&[22, 3, 1, 0]),
            ProtocolClassification::Unknown
        );
    }

    #[test]
    fn classifies_http2_when_input_is_prefix() {
        assert_eq!(
            classify_protocol_bytes(&H2_PREFACE[..8]),
            ProtocolClassification::Http2Preface
        );
    }
}
