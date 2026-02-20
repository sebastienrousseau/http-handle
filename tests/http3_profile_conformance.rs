//! HTTP/3 profile conformance tests for ALPN routing and fallback behavior.

#[cfg(feature = "http3-profile")]
use http_handle::http3_profile::{
    Http3ProductionProfile, ProtocolRoute, RouteReason,
};

#[cfg(feature = "http3-profile")]
#[test]
fn alpn_matrix_route_resolution_is_stable() {
    let profile = Http3ProductionProfile::production_baseline();
    let cases = [
        (Some(b"h3".as_slice()), true, ProtocolRoute::Http3),
        (Some(b"h2".as_slice()), true, ProtocolRoute::Http2),
        (Some(b"http/1.1".as_slice()), true, ProtocolRoute::Http11),
        (Some(b"unknown".as_slice()), true, ProtocolRoute::Http11),
        (None, true, ProtocolRoute::Http11),
    ];

    for (alpn, h3_ok, expected) in cases {
        let decision = profile.resolve_route(alpn, h3_ok);
        assert_eq!(decision.selected, expected);
    }
}

#[cfg(feature = "http3-profile")]
#[test]
fn h3_handshake_failure_falls_back_to_h2() {
    let profile = Http3ProductionProfile::production_baseline();
    let decision = profile.resolve_route(Some(b"h3"), false);
    assert_eq!(decision.selected, ProtocolRoute::Http2);
    assert_eq!(decision.reason, RouteReason::H3HandshakeFailedFallback);
}

#[cfg(feature = "http3-profile")]
#[test]
fn client_offered_alpn_selection_prefers_server_policy() {
    let profile = Http3ProductionProfile {
        alpn_order: vec!["h2".into(), "h3".into(), "http/1.1".into()],
        ..Http3ProductionProfile::production_baseline()
    };
    let client =
        vec![b"h3".to_vec(), b"h2".to_vec(), b"http/1.1".to_vec()];
    assert_eq!(
        profile.route_for_client_alpns(&client),
        ProtocolRoute::Http2
    );
}
