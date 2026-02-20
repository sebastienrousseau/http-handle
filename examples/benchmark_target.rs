//! Benchmark target server used by CI performance matrix.

use http_handle::Server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::var("HTTP_HANDLE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8090".to_string());
    let root = std::env::var("HTTP_HANDLE_ROOT")
        .unwrap_or_else(|_| ".".to_string());
    let mode = std::env::var("HTTP_HANDLE_MODE")
        .unwrap_or_else(|_| "sync".to_string());

    let server = Server::builder()
        .address(&address)
        .document_root(&root)
        .static_cache_ttl_secs(300)
        .build()?;

    match mode.as_str() {
        "sync" => server.start()?,
        "async" => run_async(server)?,
        "high-perf" => run_high_perf(server)?,
        "http2" => run_http2(server)?,
        other => {
            return Err(format!("unsupported mode: {other}").into());
        }
    }
    Ok(())
}

fn run_async(server: Server) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "async")]
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime
            .block_on(http_handle::async_server::start_async(server))?;
        Ok(())
    }
    #[cfg(not(feature = "async"))]
    {
        let _ = server;
        Err("enable feature 'async' for HTTP_HANDLE_MODE=async".into())
    }
}

fn run_high_perf(
    server: Server,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "high-perf")]
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let limits = http_handle::perf_server::PerfLimits {
            max_inflight: 512,
            max_queue: 2048,
            sendfile_threshold_bytes: 64 * 1024,
        };
        runtime.block_on(http_handle::perf_server::start_high_perf(
            server, limits,
        ))?;
        Ok(())
    }
    #[cfg(not(feature = "high-perf"))]
    {
        let _ = server;
        Err("enable feature 'high-perf' for HTTP_HANDLE_MODE=high-perf"
            .into())
    }
}

fn run_http2(server: Server) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "http2")]
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime
            .block_on(http_handle::http2_server::start_http2(server))?;
        Ok(())
    }
    #[cfg(not(feature = "http2"))]
    {
        let _ = server;
        Err("enable feature 'http2' for HTTP_HANDLE_MODE=http2".into())
    }
}
