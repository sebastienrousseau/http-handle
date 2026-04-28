// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `LanguageDetector`: built-in + runtime custom regex patterns.
//!
//! Run: `cargo run --example language`

#[path = "support.rs"]
mod support;

use http_handle::{Language, LanguageDetector};

fn main() {
    support::header("http-handle -- language");

    support::task_with_output(
        "Detect via the built-in pattern set",
        || {
            let detector = LanguageDetector::new();
            let cases = [
                ("fn main() { println!(\"hi\"); }", "Rust"),
                ("def main():\n    print('hi')", "Python"),
                ("function main() { console.log('hi'); }", "JS-ish"),
            ];
            cases
                .into_iter()
                .map(|(snippet, hint)| {
                    let detected = detector.detect(snippet);
                    format!(
                        "{hint:<8} -> {} ({:?})",
                        detected.as_str(),
                        snippet
                    )
                })
                .collect()
        },
    );

    support::task_with_output(
        "Register a custom Rust pattern at runtime",
        || {
            let detector = LanguageDetector::new()
                .with_custom_pattern(Language::Rust, r"\bcrate::\w+")
                .expect("valid regex");
            let detected =
                detector.detect("use crate::server::Server;");
            vec![format!("custom-pattern -> {}", detected.as_str())]
        },
    );

    support::summary(2);
}
