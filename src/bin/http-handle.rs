// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Minimal `http-handle` executable used for release artifacts.

use http_handle::Server;

fn runtime_settings_with(
    getter: impl Fn(&str) -> Option<String>,
) -> (String, String) {
    let address = getter("HTTP_HANDLE_ADDR")
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());
    let root =
        getter("HTTP_HANDLE_ROOT").unwrap_or_else(|| ".".to_string());
    (address, root)
}

fn build_server(
    address: &str,
    root: &str,
) -> Result<Server, Box<dyn std::error::Error>> {
    let server = Server::builder()
        .address(address)
        .document_root(root)
        .build()?;
    Ok(server)
}

fn run_with(
    getter: impl Fn(&str) -> Option<String>,
    starter: impl FnOnce(Server) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (address, root) = runtime_settings_with(getter);
    let server = build_server(&address, &root)?;
    starter(server)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_with(
        |name| std::env::var(name).ok(),
        |server| {
            server.start().map_err(|e| -> Box<dyn std::error::Error> {
                Box::new(e)
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_settings_default_when_env_missing() {
        let (addr, root) = runtime_settings_with(|_| None);
        assert_eq!(addr, "0.0.0.0:8080");
        assert_eq!(root, ".");
    }

    #[test]
    fn runtime_settings_use_custom_values() {
        let (addr, root) = runtime_settings_with(|k| match k {
            "HTTP_HANDLE_ADDR" => Some("127.0.0.1:9999".to_string()),
            "HTTP_HANDLE_ROOT" => Some("/tmp".to_string()),
            _ => None,
        });
        assert_eq!(addr, "127.0.0.1:9999");
        assert_eq!(root, "/tmp");
    }

    #[test]
    fn build_server_keeps_configured_address() {
        let server =
            build_server("invalid-address", ".").expect("server");
        assert_eq!(server.address(), "invalid-address");
    }

    #[test]
    fn build_server_accepts_valid_settings() {
        let server = build_server("127.0.0.1:0", ".").expect("server");
        assert_eq!(server.address(), "127.0.0.1:0");
    }

    #[test]
    fn run_with_invokes_starter() {
        let ran = std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        );
        let flag = ran.clone();
        run_with(
            |_| None,
            move |_server| {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        )
        .expect("run");
        assert!(ran.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn run_with_propagates_starter_error() {
        let err = run_with(
            |_| None,
            |_server| Err("starter failed".to_string().into()),
        )
        .expect_err("starter should fail");
        assert!(err.to_string().contains("starter failed"));
    }

    #[test]
    fn main_returns_error_for_invalid_bind_address() {
        // Safety: bounded test-only process env mutation.
        unsafe { std::env::set_var("HTTP_HANDLE_ADDR", "not-an-addr") };
        let result = main();
        // Safety: paired cleanup for env key set above.
        unsafe { std::env::remove_var("HTTP_HANDLE_ADDR") };
        assert!(result.is_err());
    }
}
