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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (address, root) =
        runtime_settings_with(|name| std::env::var(name).ok());
    let server = build_server(&address, &root)?;

    server.start()?;
    Ok(())
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
}
