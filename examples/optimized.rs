// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `optimized` feature: const MIME table + bitset language detection.
//!
//! Run: `cargo run --example optimized --features optimized`

#[cfg(feature = "optimized")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "optimized")]
fn main() {
    use http_handle::Language;
    use http_handle::optimized::{
        LanguageSet, const_content_type_from_ext, detect_language_fast,
    };

    support::header("http-handle -- optimized");

    support::task_with_output(
        "LanguageSet is a deduplicating bitset, not a HashSet",
        || {
            let mut set = LanguageSet::new();
            set.insert(Language::Rust);
            set.insert(Language::Python);
            set.insert(Language::Rust); // duplicate is a no-op
            vec![
                format!("size           = {}", set.as_slice().len()),
                format!(
                    "contains rust  = {}",
                    set.as_slice().contains(&Language::Rust)
                ),
            ]
        },
    );

    support::task_with_output(
        "const_content_type_from_ext is a compile-time match table",
        || {
            ["html", "wasm", "json", "xyz"]
                .into_iter()
                .map(|ext| {
                    format!(
                        "{ext:<5} -> {}",
                        const_content_type_from_ext(ext)
                    )
                })
                .collect()
        },
    );

    support::task_with_output(
        "detect_language_fast trims allocator pressure on the hot path",
        || {
            let detected =
                detect_language_fast("fn main() { println!(\"hi\"); }");
            vec![format!("rust snippet -> {}", detected.as_str())]
        },
    );

    support::summary(3);
}

#[cfg(not(feature = "optimized"))]
fn main() {
    eprintln!(
        "Enable the 'optimized' feature: cargo run --example optimized --features optimized"
    );
}
