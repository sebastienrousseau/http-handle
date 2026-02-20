//! Language detection example with runtime custom patterns.

use http_handle::{Language, LanguageDetector};

fn main() {
    let detector = LanguageDetector::new()
        .with_custom_pattern(Language::Rust, r"\bcrate::\w+")
        .expect("valid regex pattern");

    let sample = "use crate::server::Server;";
    let detected = detector.detect(sample);

    println!("Sample: {sample}");
    println!("Detected language: {}", detected.as_str());
}
