// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `Response`: build, serialise, and exercise `set_connection_header`.
//!
//! Run: `cargo run --example response`

#[path = "support.rs"]
mod support;

use http_handle::response::Response;
use std::io::Cursor;

fn main() {
    support::header("http-handle -- response");

    support::task_with_output(
        "200 OK with HTML body and headers",
        || {
            let mut response =
                Response::new(200, "OK", b"<h1>Hello</h1>".to_vec());
            response.add_header("Content-Type", "text/html");
            response.add_header(
                "Content-Length",
                &response.body.len().to_string(),
            );

            let mut sink = Cursor::new(Vec::<u8>::new());
            response.send(&mut sink).expect("send");
            let wire =
                String::from_utf8_lossy(sink.get_ref()).into_owned();
            wire.lines().take(5).map(str::to_string).collect()
        },
    );

    support::task_with_output(
        "404 with empty body adds default Connection",
        || {
            let response = Response::new(404, "Not Found", Vec::new());
            let mut sink = Cursor::new(Vec::<u8>::new());
            response.send(&mut sink).expect("send");
            let wire =
                String::from_utf8_lossy(sink.get_ref()).into_owned();
            wire.lines().map(str::to_string).take(4).collect()
        },
    );

    support::task_with_output(
        "set_connection_header replaces any existing value",
        || {
            let mut response = Response::new(200, "OK", Vec::new());
            response.add_header("Connection", "close");
            response.set_connection_header("keep-alive");
            let connection = response
                .headers
                .iter()
                .find(|(name, _)| {
                    name.eq_ignore_ascii_case("Connection")
                })
                .map(|(_, value)| value.clone())
                .unwrap_or_default();
            vec![format!("Connection: {connection}")]
        },
    );

    support::summary(3);
}
