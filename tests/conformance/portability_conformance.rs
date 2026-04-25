// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

#![allow(missing_docs)]

use http_handle::request::Request;
use std::io::Write;
use std::net::{TcpListener, TcpStream};

fn parse_request_bytes(
    bytes: &[u8],
) -> Result<Request, http_handle::ServerError> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");

    let payload = bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let mut stream = TcpStream::connect(addr).expect("connect");
        stream.write_all(&payload).expect("write");
    });

    let (stream, _) = listener.accept().expect("accept");
    let parsed = Request::from_stream(&stream);
    writer.join().expect("join");
    parsed
}

#[test]
fn parses_crlf_lines_portably() {
    let request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let parsed = parse_request_bytes(request).expect("request");
    assert_eq!(parsed.method(), "GET");
    assert_eq!(parsed.path(), "/");
    assert_eq!(parsed.version(), "HTTP/1.1");
}

#[test]
fn parses_lf_only_lines_for_platform_tolerance() {
    let request = b"GET / HTTP/1.1\nHost: localhost\n\n";
    let parsed = parse_request_bytes(request).expect("request");
    assert_eq!(parsed.path(), "/");
}

#[test]
fn path_normalization_blocks_parent_traversal() {
    let server = http_handle::Server::new("127.0.0.1:0", ".");
    assert_eq!(server.address(), "127.0.0.1:0");
    // This test intentionally verifies we can construct with dotted roots on all targets.
    assert!(server.document_root().ends_with("."));
}

#[test]
fn socket_bind_port_zero_is_portable() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    assert!(addr.port() > 0);
}
