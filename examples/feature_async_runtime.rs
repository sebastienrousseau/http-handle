//! Hardened async runtime helper example.

#[cfg(feature = "async")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = http_handle::async_runtime::run_blocking(|| {
        let answer = 6 * 7;
        Ok::<_, http_handle::ServerError>(answer)
    })
    .await?;

    println!("run_blocking returned: {value}");
    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!(
        "Enable the 'async' feature: cargo run --example feature_async_runtime --features async"
    );
}
