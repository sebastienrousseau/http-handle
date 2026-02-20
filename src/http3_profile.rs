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

/// Runtime QUIC tuning preset.
#[cfg(feature = "http3-profile")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuicTuningPreset {
    /// Lower resource use and conservative timeouts.
    Conservative,
    /// Balanced defaults for general production use.
    Balanced,
    /// Throughput-biased tuning for high-capacity edge deployments.
    Aggressive,
}

/// Derived QUIC runtime tuning values.
#[cfg(feature = "http3-profile")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuicTuning {
    /// QUIC idle timeout in milliseconds.
    pub idle_timeout_ms: u64,
    /// Keep-alive probe interval in milliseconds.
    pub keep_alive_interval_ms: u64,
    /// Max concurrent bidirectional streams.
    pub max_bidi_streams: u64,
    /// Datagram receive buffer target in bytes.
    pub datagram_receive_buffer_bytes: usize,
}

/// Reason describing how route selection was resolved.
#[cfg(feature = "http3-profile")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteReason {
    /// Standard negotiated protocol route.
    Negotiated,
    /// HTTP/3 profile disabled.
    H3Disabled,
    /// ALPN missing during negotiation.
    AlpnMissing,
    /// ALPN provided but not recognized.
    AlpnUnsupported,
    /// H3 handshake failed and fallback was applied.
    H3HandshakeFailedFallback,
    /// H3 handshake failed and fallback is disabled.
    H3HandshakeFailedNoFallback,
}

/// Decision output for ALPN+fallback route resolution.
#[cfg(feature = "http3-profile")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteDecision {
    /// Final selected route.
    pub selected: ProtocolRoute,
    /// Resolution reason.
    pub reason: RouteReason,
    /// Raw negotiated ALPN token if present.
    pub negotiated_alpn: Option<String>,
    /// Ordered chain considered for fallback.
    pub fallback_chain: Vec<ProtocolRoute>,
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
    /// QUIC tuning preset.
    pub quic_preset: QuicTuningPreset,
    /// Accept draft h3 tokens (for example `h3-29`) as h3 route.
    pub allow_h3_draft: bool,
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
            quic_preset: QuicTuningPreset::Balanced,
            allow_h3_draft: true,
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

    /// Returns effective QUIC tuning values from preset and profile.
    pub fn quic_tuning(&self) -> QuicTuning {
        match self.quic_preset {
            QuicTuningPreset::Conservative => QuicTuning {
                idle_timeout_ms: self.quic_idle_timeout_ms.max(45_000),
                keep_alive_interval_ms: 15_000,
                max_bidi_streams: 64,
                datagram_receive_buffer_bytes: 512 * 1024,
            },
            QuicTuningPreset::Balanced => QuicTuning {
                idle_timeout_ms: self.quic_idle_timeout_ms.max(30_000),
                keep_alive_interval_ms: 10_000,
                max_bidi_streams: 128,
                datagram_receive_buffer_bytes: 1024 * 1024,
            },
            QuicTuningPreset::Aggressive => QuicTuning {
                idle_timeout_ms: self.quic_idle_timeout_ms.max(20_000),
                keep_alive_interval_ms: 8_000,
                max_bidi_streams: 256,
                datagram_receive_buffer_bytes: 2 * 1024 * 1024,
            },
        }
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
            Some(raw)
                if self.allow_h3_draft
                    && std::str::from_utf8(raw)
                        .map(|v| v.starts_with("h3-"))
                        .unwrap_or(false) =>
            {
                ProtocolRoute::Http3
            }
            _ => ProtocolRoute::Http11,
        }
    }

    /// Selects a route from offered client ALPN tokens and server preference.
    pub fn route_for_client_alpns(
        &self,
        client_offered_alpns: &[Vec<u8>],
    ) -> ProtocolRoute {
        if !self.enabled {
            return ProtocolRoute::Http11;
        }
        let offered = client_offered_alpns
            .iter()
            .map(|v| self.route_for_alpn(Some(v)).to_string())
            .collect::<Vec<_>>();
        for preferred in self.fallback_chain() {
            if offered.iter().any(|v| v == &preferred.to_string()) {
                return preferred;
            }
        }
        ProtocolRoute::Http11
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

    /// Resolves final route with explicit fallback decision tree and reason.
    pub fn resolve_route(
        &self,
        negotiated_alpn: Option<&[u8]>,
        h3_handshake_ok: bool,
    ) -> RouteDecision {
        let chain = self.fallback_chain();
        let negotiated = negotiated_alpn
            .map(|v| String::from_utf8_lossy(v).to_string());
        let mut selected = self.route_for_alpn(negotiated_alpn);
        let mut reason = if !self.enabled {
            RouteReason::H3Disabled
        } else {
            match negotiated_alpn {
                None => RouteReason::AlpnMissing,
                Some(b"h3") | Some(b"h2") | Some(b"http/1.1") => {
                    RouteReason::Negotiated
                }
                Some(raw)
                    if self.allow_h3_draft
                        && std::str::from_utf8(raw)
                            .map(|v| v.starts_with("h3-"))
                            .unwrap_or(false) =>
                {
                    RouteReason::Negotiated
                }
                Some(_) => RouteReason::AlpnUnsupported,
            }
        };

        if selected == ProtocolRoute::Http3 && !h3_handshake_ok {
            if self.fallback_on_h3_error {
                selected = chain
                    .iter()
                    .copied()
                    .find(|r| *r != ProtocolRoute::Http3)
                    .unwrap_or(ProtocolRoute::Http11);
                reason = RouteReason::H3HandshakeFailedFallback;
            } else {
                selected = ProtocolRoute::Http11;
                reason = RouteReason::H3HandshakeFailedNoFallback;
            }
        }

        RouteDecision {
            selected,
            reason,
            negotiated_alpn: negotiated,
            fallback_chain: chain,
        }
    }

    /// Serializes a compact fallback telemetry line for logs.
    pub fn telemetry_line(&self, decision: &RouteDecision) -> String {
        format!(
            "http3.route={} reason={} negotiated={} chain={}",
            decision.selected,
            decision.reason,
            decision.negotiated_alpn.as_deref().unwrap_or("none"),
            decision
                .fallback_chain
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(">")
        )
    }
}

#[cfg(feature = "http3-profile")]
impl std::fmt::Display for ProtocolRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ProtocolRoute::Http3 => "h3",
            ProtocolRoute::Http2 => "h2",
            ProtocolRoute::Http11 => "http/1.1",
        };
        write!(f, "{s}")
    }
}

#[cfg(feature = "http3-profile")]
impl std::fmt::Display for RouteReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RouteReason::Negotiated => "negotiated",
            RouteReason::H3Disabled => "h3_disabled",
            RouteReason::AlpnMissing => "alpn_missing",
            RouteReason::AlpnUnsupported => "alpn_unsupported",
            RouteReason::H3HandshakeFailedFallback => {
                "h3_handshake_failed_fallback"
            }
            RouteReason::H3HandshakeFailedNoFallback => {
                "h3_handshake_failed_no_fallback"
            }
        };
        write!(f, "{s}")
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
        assert_eq!(
            p.route_for_alpn(Some(b"h3-29")),
            ProtocolRoute::Http3
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

    #[test]
    fn route_for_client_alpns_respects_server_order() {
        let p = Http3ProductionProfile {
            alpn_order: vec![
                "h2".into(),
                "h3".into(),
                "http/1.1".into(),
            ],
            ..Http3ProductionProfile::default()
        };
        let client = vec![b"h3".to_vec(), b"h2".to_vec()];
        assert_eq!(
            p.route_for_client_alpns(&client),
            ProtocolRoute::Http2
        );
    }

    #[test]
    fn resolve_route_falls_back_on_h3_handshake_failure() {
        let p = Http3ProductionProfile::default();
        let decision = p.resolve_route(Some(b"h3"), false);
        assert_eq!(decision.selected, ProtocolRoute::Http2);
        assert_eq!(
            decision.reason,
            RouteReason::H3HandshakeFailedFallback
        );
    }

    #[test]
    fn resolve_route_handles_no_fallback_mode() {
        let p = Http3ProductionProfile {
            fallback_on_h3_error: false,
            ..Http3ProductionProfile::default()
        };
        let decision = p.resolve_route(Some(b"h3"), false);
        assert_eq!(decision.selected, ProtocolRoute::Http11);
        assert_eq!(
            decision.reason,
            RouteReason::H3HandshakeFailedNoFallback
        );
    }

    #[test]
    fn quic_preset_changes_tuning_envelope() {
        let conservative = Http3ProductionProfile {
            quic_preset: QuicTuningPreset::Conservative,
            ..Http3ProductionProfile::default()
        }
        .quic_tuning();
        let aggressive = Http3ProductionProfile {
            quic_preset: QuicTuningPreset::Aggressive,
            ..Http3ProductionProfile::default()
        }
        .quic_tuning();
        assert!(
            aggressive.max_bidi_streams > conservative.max_bidi_streams
        );
        assert!(
            aggressive.datagram_receive_buffer_bytes
                > conservative.datagram_receive_buffer_bytes
        );
    }

    #[test]
    fn telemetry_line_contains_decision_fields() {
        let p = Http3ProductionProfile::default();
        let decision = p.resolve_route(Some(b"h2"), true);
        let line = p.telemetry_line(&decision);
        assert!(line.contains("http3.route=h2"));
        assert!(line.contains("reason=negotiated"));
        assert!(line.contains("chain=h3>h2>http/1.1"));
    }
}
