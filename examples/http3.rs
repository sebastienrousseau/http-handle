// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `http3-profile` feature: ALPN routing, fallback chain, QUIC tuning.
//!
//! Run: `cargo run --example http3 --features http3-profile`

#[cfg(feature = "http3-profile")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "http3-profile")]
fn main() {
    use http_handle::http3_profile::{
        Http3ProductionProfile, ProtocolRoute, QuicTuningPreset,
    };

    support::header("http-handle -- http3");

    let profile = Http3ProductionProfile {
        quic_preset: QuicTuningPreset::Balanced,
        ..Http3ProductionProfile::production_baseline()
    };

    support::task_with_output("ALPN-driven route selection", || {
        let h3_29 = profile.route_for_alpn(Some(b"h3-29"));
        let h3 = profile.route_for_alpn(Some(b"h3"));
        let unknown = profile.route_for_alpn(Some(b"some-future"));
        vec![
            format!("h3-29       -> {h3_29:?}"),
            format!("h3          -> {h3:?}"),
            format!("future-alpn -> {unknown:?}"),
        ]
    });

    support::task_with_output(
        "Fallback chain favours HTTP/3 then degrades cleanly",
        || {
            let chain = profile.fallback_chain();
            assert_eq!(chain.first(), Some(&ProtocolRoute::Http3));
            chain
                .iter()
                .enumerate()
                .map(|(i, r)| format!("{i}. {r:?}"))
                .collect()
        },
    );

    support::task_with_output(
        "resolve_route + telemetry_line for negotiated traffic",
        || {
            let decision = profile.resolve_route(Some(b"h3"), false);
            vec![
                format!("decision  = {decision:?}"),
                format!(
                    "telemetry = {}",
                    profile.telemetry_line(&decision)
                ),
            ]
        },
    );

    support::task_with_output(
        "QUIC tuning preset baked into the profile",
        || vec![format!("{:?}", profile.quic_tuning())],
    );

    support::summary(4);
}

#[cfg(not(feature = "http3-profile"))]
fn main() {
    eprintln!(
        "Enable the 'http3-profile' feature: cargo run --example http3 --features http3-profile"
    );
}
