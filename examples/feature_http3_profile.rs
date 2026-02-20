//! Example showing HTTP/3 ALPN routing and fallback policy.

#[cfg(feature = "http3-profile")]
use http_handle::http3_profile::{
    Http3ProductionProfile, ProtocolRoute,
};

fn main() {
    #[cfg(feature = "http3-profile")]
    {
        let profile = Http3ProductionProfile::production_baseline();
        let route = profile.route_for_alpn(Some(b"h3"));
        println!("Negotiated route: {:?}", route);
        let chain = profile.fallback_chain();
        assert_eq!(chain.first(), Some(&ProtocolRoute::Http3));
        println!("Fallback chain: {:?}", chain);
    }
}
