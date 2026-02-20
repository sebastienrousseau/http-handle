//! Observability initialization example.

#[cfg(feature = "observability")]
fn main() {
    use http_handle::observability::init_tracing;
    use tracing::info;

    init_tracing();
    info!(target: "http_handle::example", "tracing initialized");
    println!(
        "Tracing initialized. Set RUST_LOG=info to see structured logs."
    );
}

#[cfg(not(feature = "observability"))]
fn main() {
    eprintln!(
        "Enable the 'observability' feature: cargo run --example feature_observability --features observability"
    );
}
