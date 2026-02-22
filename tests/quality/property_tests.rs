// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Property-based tests for request parsing, content behavior, and language detection.

use http_handle::{Language, LanguageDetector};
use proptest::prelude::*;
use std::io::Write;
use std::net::{TcpListener, TcpStream};

fn parse_request_line(line: String) -> Result<String, String> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;

    let send_line = line.clone();
    let _ = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.write_all(send_line.as_bytes());
        }
    });

    let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    http_handle::request::Request::from_stream(&stream)
        .map(|req| req.to_string())
        .map_err(|e| e.to_string())
}

proptest! {
    #[test]
    fn invalid_methods_are_rejected(method in "[A-Z]{1,10}") {
        prop_assume!(!matches!(method.as_str(), "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH"));
        let line = format!("{} / HTTP/1.1\r\n", method);
        let parsed = parse_request_line(line);
        prop_assert!(parsed.is_err());
    }

    #[test]
    fn options_star_is_valid(version in prop_oneof![Just("HTTP/1.0"), Just("HTTP/1.1")]) {
        let line = format!("OPTIONS * {}\r\n", version);
        let parsed = parse_request_line(line).expect("request should parse");
        prop_assert!(parsed.contains("OPTIONS *"));
    }

    #[test]
    fn custom_pattern_matches_runtime_literals(token in "[a-z]{3,12}") {
        let pattern = format!(r"\b{}\b", token);
        let input = format!("prefix {} suffix", token);
        let baseline = LanguageDetector::new().detect(&input);
        let detector = LanguageDetector::new().with_custom_pattern(Language::Go, &pattern).expect("regex compiles");
        let detected = detector.detect(&input);
        if baseline == Language::Unknown {
            prop_assert_eq!(detected, Language::Go);
        } else {
            // Custom rules are appended; built-in higher-priority matches stay stable.
            prop_assert_eq!(detected, baseline);
        }
    }
}
