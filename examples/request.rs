// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `Request::from_stream`: loopback parse of a real TCP request.
//!
//! Run: `cargo run --example request`

#[path = "support.rs"]
mod support;

use http_handle::request::Request;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn main() {
    support::header("http-handle -- request");

    let listener =
        support::task("Bind ephemeral loopback listener", || {
            TcpListener::bind("127.0.0.1:0").expect("bind")
        });
    let addr = listener.local_addr().expect("addr");

    let _client = thread::spawn(move || {
        // Settle so accept() is parked when we connect.
        thread::sleep(Duration::from_millis(50));
        let mut s = TcpStream::connect(addr).expect("connect");
        s.write_all(
            b"GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nUser-Agent: example\r\n\r\n",
        )
        .expect("write");
    });

    support::task_with_output(
        "Parse one request from the stream",
        || {
            let (stream, _) = listener.accept().expect("accept");
            let request = Request::from_stream(&stream).expect("parse");
            vec![
                format!("method  = {}", request.method()),
                format!("path    = {}", request.path()),
                format!("version = {}", request.version()),
                format!(
                    "host    = {}",
                    request.header("host").unwrap_or("(none)")
                ),
                format!("rendered = {request}"),
            ]
        },
    );

    support::task_with_output(
        "Header lookup is case-insensitive",
        || {
            // Construct a request literal so we can demonstrate the
            // accessor without spinning a second TCP roundtrip.
            let request = Request {
                method: "POST".into(),
                path: "/api".into(),
                version: "HTTP/1.1".into(),
                headers: vec![
                    ("Content-Type".into(), "application/json".into()),
                    ("X-Trace-Id".into(), "abc-123".into()),
                ],
            };
            vec![
                format!(
                    "header(\"content-type\") = {}",
                    request.header("content-type").unwrap_or("(none)")
                ),
                format!(
                    "header(\"X-TRACE-ID\")   = {}",
                    request.header("X-TRACE-ID").unwrap_or("(none)")
                ),
            ]
        },
    );

    support::summary(3);
}
