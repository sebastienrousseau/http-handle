//! Zero-allocation lookup helpers for hot-path operations.

use crate::language::Language;

/// Small stack-backed language set.
///
/// # Examples
///
/// ```rust
/// use http_handle::optimized::LanguageSet;
/// let set = LanguageSet::new();
/// assert!(set.as_slice().is_empty());
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageSet {
    data: [Language; 8],
    len: usize,
}

impl LanguageSet {
    /// Creates an empty set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::optimized::LanguageSet;
    /// let set = LanguageSet::new();
    /// assert_eq!(set.as_slice().len(), 0);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub const fn new() -> Self {
        Self {
            data: [Language::Unknown; 8],
            len: 0,
        }
    }

    /// Inserts a language if absent and capacity allows.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::Language;
    /// use http_handle::optimized::LanguageSet;
    /// let mut set = LanguageSet::new();
    /// set.insert(Language::Rust);
    /// assert!(set.contains(Language::Rust));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn insert(&mut self, language: Language) {
        if self.contains(language) || self.len >= self.data.len() {
            return;
        }
        self.data[self.len] = language;
        self.len += 1;
    }

    /// Returns true if language is present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::Language;
    /// use http_handle::optimized::LanguageSet;
    /// let mut set = LanguageSet::new();
    /// set.insert(Language::Go);
    /// assert!(set.contains(Language::Go));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn contains(&self, language: Language) -> bool {
        self.data[..self.len].contains(&language)
    }

    /// Returns slice view over inserted languages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::Language;
    /// use http_handle::optimized::LanguageSet;
    /// let mut set = LanguageSet::new();
    /// set.insert(Language::Python);
    /// assert_eq!(set.as_slice(), &[Language::Python]);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn as_slice(&self) -> &[Language] {
        &self.data[..self.len]
    }
}

impl Default for LanguageSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Branch-optimized extension to content type lookup for hot paths.
///
/// # Examples
///
/// ```rust
/// use http_handle::optimized::const_content_type_from_ext;
/// assert_eq!(const_content_type_from_ext("wasm"), "application/wasm");
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn const_content_type_from_ext(ext: &str) -> &'static str {
    match ext {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "webp" => "image/webp",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

/// Fast no-allocation language hinting using substring checks only.
///
/// # Examples
///
/// ```rust
/// use http_handle::language::Language;
/// use http_handle::optimized::detect_language_fast;
/// assert_eq!(detect_language_fast("fn main() {}"), Language::Rust);
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn detect_language_fast(input: &str) -> Language {
    if input.contains("fn ")
        || input.contains("impl ")
        || input.contains("let ")
    {
        return Language::Rust;
    }
    if input.contains("def ") || input.contains("import ") {
        return Language::Python;
    }
    if input.contains("function ")
        || input.contains("const ")
        || input.contains("=>")
    {
        return Language::JavaScript;
    }
    if input.contains("package ") || input.contains("func ") {
        return Language::Go;
    }
    Language::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_is_stack_based_and_deduplicated() {
        let mut set = LanguageSet::new();
        set.insert(Language::Rust);
        set.insert(Language::Rust);
        set.insert(Language::Python);
        assert_eq!(set.as_slice(), &[Language::Rust, Language::Python]);
    }

    #[test]
    fn detects_const_content_types() {
        assert_eq!(
            const_content_type_from_ext("wasm"),
            "application/wasm"
        );
        assert_eq!(
            const_content_type_from_ext("unknown"),
            "application/octet-stream"
        );
    }

    #[test]
    fn detects_fast_language_paths() {
        assert_eq!(
            detect_language_fast("fn main() {}"),
            Language::Rust
        );
        assert_eq!(
            detect_language_fast("def main(): pass"),
            Language::Python
        );
        assert_eq!(
            detect_language_fast("const x = () => 1;"),
            Language::JavaScript
        );
        assert_eq!(
            detect_language_fast("package main\nfunc main() {}"),
            Language::Go
        );
        assert_eq!(
            detect_language_fast("plain text"),
            Language::Unknown
        );
    }

    #[test]
    fn set_capacity_is_bounded() {
        let mut set = LanguageSet::new();
        for _ in 0..16 {
            set.insert(Language::Rust);
            set.insert(Language::Python);
            set.insert(Language::JavaScript);
            set.insert(Language::Go);
            set.insert(Language::Unknown);
        }
        assert!(set.as_slice().len() <= 8);
    }

    #[test]
    fn default_matches_new_set() {
        let via_default = LanguageSet::default();
        let via_new = LanguageSet::new();
        assert_eq!(via_default.as_slice(), via_new.as_slice());
    }
}
