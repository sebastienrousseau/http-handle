// Simple test script to verify MIME type support
use std::path::Path;

fn get_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(std::ffi::OsStr::to_str) {
        // Modern image formats
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",

        // Web Assembly
        Some("wasm") => "application/wasm",

        // Default fallback
        _ => "application/octet-stream",
    }
}

fn main() {
    println!("Testing modern MIME types:");

    // Test WebP
    let webp_result = get_content_type(Path::new("test.webp"));
    println!("webp: {} (expected: image/webp)", webp_result);
    assert_eq!(webp_result, "image/webp");

    // Test AVIF
    let avif_result = get_content_type(Path::new("test.avif"));
    println!("avif: {} (expected: image/avif)", avif_result);
    assert_eq!(avif_result, "image/avif");

    // Test WASM
    let wasm_result = get_content_type(Path::new("module.wasm"));
    println!("wasm: {} (expected: application/wasm)", wasm_result);
    assert_eq!(wasm_result, "application/wasm");

    println!("All modern MIME types working correctly! ✅");
}