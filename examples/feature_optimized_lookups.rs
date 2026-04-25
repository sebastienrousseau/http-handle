// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Zero-cost optimization helpers example.

#[cfg(feature = "optimized")]
fn main() {
    use http_handle::Language;
    use http_handle::optimized::{
        LanguageSet, const_content_type_from_ext, detect_language_fast,
    };

    let mut set = LanguageSet::new();
    set.insert(Language::Rust);
    set.insert(Language::Python);
    set.insert(Language::Rust);

    let ext = "wasm";
    let mime = const_content_type_from_ext(ext);
    let lang = detect_language_fast("fn main() { println!(\"hi\"); }");

    println!("language set size: {}", set.as_slice().len());
    println!("{ext} => {mime}");
    println!("fast language detection => {}", lang.as_str());
}

#[cfg(not(feature = "optimized"))]
fn main() {
    eprintln!(
        "Enable the 'optimized' feature: cargo run --example feature_optimized_lookups --features optimized"
    );
}
