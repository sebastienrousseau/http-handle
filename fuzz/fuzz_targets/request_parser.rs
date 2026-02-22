// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Write;
use std::net::{TcpListener, TcpStream};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() || data.len() > 4096 {
        return;
    }

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(_) => return,
    };
    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(_) => return,
    };

    let payload = data.to_vec();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.write_all(&payload);
        }
    });

    if let Ok(stream) = TcpStream::connect(addr) {
        let _ = http_handle::request::Request::from_stream(&stream);
    }
});
