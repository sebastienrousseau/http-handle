//! HTTP/3 production profile primitives.
//!
//! This module defines ALPN routing and fallback policy helpers so deployments
//! can enforce consistent behavior when HTTP/3 is enabled.

/// Effective protocol route selected after ALPN negotiation.
#[cfg(feature = "http3-profile")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolRoute {
    /// Use HTTP/3 over QUIC.
    Http3,
    /// Fallback to HTTP/2.
    Http2,
    /// Fallback to HTTP/1.1.
    Http11,
}

/// Production-focused HTTP/3 configuration profile.
#[cfg(feature = "http3-profile")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Http3ProductionProfile {
    /// Whether HTTP/3 is enabled.
    pub enabled: bool,
    /// Ordered ALPN preference, e.g. `["h3", "h2", "http/1.1"]`.
    pub alpn_order: Vec<String>,
    /// QUIC idle timeout in milliseconds.
    pub quic_idle_timeout_ms: u64,
    /// Whether failed HTTP/3 handshakes should fallback to H2/H1.
    pub fallback_on_h3_error: bool,
}

#[cfg(feature = "http3-profile")]
impl Default for Http3ProductionProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            alpn_order: vec![
                "h3".to_string(),
                "h2".to_string(),
                "http/1.1".to_string(),
            ],
            quic_idle_timeout_ms: 30_000,
            fallback_on_h3_error: true,
        }
    }
}

#[cfg(feature = "http3-profile")]
impl Http3ProductionProfile {
    /// Returns a strict production baseline with h3-first ALPN ordering.
    pub fn production_baseline() -> Self {
        Self::default()
    }

    /// Derives the serving route from negotiated ALPN protocol bytes.
    pub fn route_for_alpn(
        &self,
        negotiated_alpn: Option<&[u8]>,
    ) -> ProtocolRoute {
        if !self.enabled {
            return ProtocolRoute::Http11;
        }
        match negotiated_alpn {
            Some(b"h3") => ProtocolRoute::Http3,
            Some(b"h2") => ProtocolRoute::Http2,
            Some(b"http/1.1") => ProtocolRoute::Http11,
            _ => ProtocolRoute::Http11,
        }
    }

    /// Returns ordered protocol fallback chain.
    pub fn fallback_chain(&self) -> Vec<ProtocolRoute> {
        let mut chain = Vec::new();
        for protocol in &self.alpn_order {
            let route = match protocol.as_str() {
                "h3" => ProtocolRoute::Http3,
                "h2" => ProtocolRoute::Http2,
                "http/1.1" => ProtocolRoute::Http11,
                _ => continue,
            };
            if !chain.contains(&route) {
                chain.push(route);
            }
        }
        if chain.is_empty() {
            chain.push(ProtocolRoute::Http11);
        }
        chain
    }
}

#[cfg(all(test, feature = "http3-profile"))]
mod tests {
    use super::*;

    #[test]
    fn production_baseline_prefers_h3() {
        let p = Http3ProductionProfile::production_baseline();
        assert!(p.enabled);
        assert_eq!(p.alpn_order[0], "h3");
        assert!(p.fallback_on_h3_error);
    }

    #[test]
    fn route_for_alpn_handles_known_protocols() {
        let p = Http3ProductionProfile::default();
        assert_eq!(p.route_for_alpn(Some(b"h3")), ProtocolRoute::Http3);
        assert_eq!(p.route_for_alpn(Some(b"h2")), ProtocolRoute::Http2);
        assert_eq!(
            p.route_for_alpn(Some(b"http/1.1")),
            ProtocolRoute::Http11
        );
        assert_eq!(p.route_for_alpn(None), ProtocolRoute::Http11);
    }

    #[test]
    fn fallback_chain_is_unique_and_ordered() {
        let p = Http3ProductionProfile {
            alpn_order: vec![
                "h3".into(),
                "h2".into(),
                "h2".into(),
                "http/1.1".into(),
            ],
            ..Http3ProductionProfile::default()
        };
        assert_eq!(
            p.fallback_chain(),
            vec![
                ProtocolRoute::Http3,
                ProtocolRoute::Http2,
                ProtocolRoute::Http11
            ]
        );
    }
}
