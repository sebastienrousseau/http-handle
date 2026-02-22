// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

//! Lightweight language detection with runtime-customizable patterns.

use regex::Regex;

/// Supported languages detected by this module.
///
/// # Examples
///
/// ```rust
/// use http_handle::language::Language;
/// assert_eq!(Language::Rust.as_str(), "rust");
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Language {
    /// Rust language.
    Rust,
    /// Python language.
    Python,
    /// JavaScript language.
    JavaScript,
    /// Go language.
    Go,
    /// Unknown or unsupported language.
    Unknown,
}

impl Language {
    /// Returns a static string identifier for the language.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::Language;
    /// assert_eq!(Language::Go.as_str(), "go");
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::Go => "go",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug)]
struct PatternRule {
    language: Language,
    pattern: Regex,
}

/// Runtime language detector with optional custom pattern rules.
///
/// # Examples
///
/// ```rust
/// use http_handle::language::{Language, LanguageDetector};
/// let detector = LanguageDetector::new();
/// assert_eq!(detector.detect("fn main() {}"), Language::Rust);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Debug)]
pub struct LanguageDetector {
    rules: Vec<PatternRule>,
}

impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageDetector {
    /// Creates a detector with built-in default patterns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::{Language, LanguageDetector};
    /// let detector = LanguageDetector::new();
    /// assert_eq!(detector.detect("def f(): pass"), Language::Python);
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics only if a built-in regex literal is invalid.
    pub fn new() -> Self {
        let defaults = [
            (Language::Rust, r"\b(fn|let|impl|pub|crate)\b"),
            (Language::Python, r"\b(def|import|lambda|async\s+def)\b"),
            (
                Language::JavaScript,
                r"\b(function|const|let|=>|console\.log)\b",
            ),
            (Language::Go, r"\b(func|package|go\s+|defer)\b"),
        ];

        let rules = defaults
            .iter()
            .map(|(language, pattern)| PatternRule {
                language: *language,
                pattern: Regex::new(pattern)
                    .expect("default language regex must compile"),
            })
            .collect();

        Self { rules }
    }

    /// Adds a runtime custom pattern to detect a specific language.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::{Language, LanguageDetector};
    /// let detector = LanguageDetector::new()
    ///     .with_custom_pattern(Language::Go, r"\\bpackage\\b")
    ///     .expect("valid regex");
    /// assert_eq!(detector.detect("package main"), Language::Go);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error when `pattern` is not a valid regular expression.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn with_custom_pattern(
        mut self,
        language: Language,
        pattern: &str,
    ) -> Result<Self, regex::Error> {
        self.rules.push(PatternRule {
            language,
            pattern: Regex::new(pattern)?,
        });
        Ok(self)
    }

    /// Detects the first matching language for a text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::language::{Language, LanguageDetector};
    /// let detector = LanguageDetector::new();
    /// assert_eq!(detector.detect("const x = 1;"), Language::JavaScript);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn detect(&self, input: &str) -> Language {
        self.rules
            .iter()
            .find(|rule| rule.pattern.is_match(input))
            .map_or(Language::Unknown, |rule| rule.language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_default_languages() {
        let detector = LanguageDetector::new();
        assert_eq!(
            detector.detect("fn main() { let x = 1; }"),
            Language::Rust
        );
        assert_eq!(
            detector.detect("def f(x): return x"),
            Language::Python
        );
    }

    #[test]
    fn supports_runtime_custom_pattern() {
        let detector = LanguageDetector::new()
            .with_custom_pattern(Language::Rust, r"\bcargo\s+build\b")
            .expect("pattern should compile");

        assert_eq!(
            detector.detect("cargo build --release"),
            Language::Rust
        );
    }

    #[test]
    fn language_as_str_is_stable() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::JavaScript.as_str(), "javascript");
        assert_eq!(Language::Go.as_str(), "go");
        assert_eq!(Language::Unknown.as_str(), "unknown");
    }

    #[test]
    fn custom_pattern_validation_errors() {
        let result = LanguageDetector::new()
            .with_custom_pattern(Language::Go, r"[");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_language_falls_back() {
        let detector = LanguageDetector::new();
        assert_eq!(
            detector.detect("just prose without code"),
            Language::Unknown
        );
    }

    #[test]
    fn default_matches_new() {
        let via_default = LanguageDetector::default();
        let via_new = LanguageDetector::new();
        assert_eq!(
            via_default.detect("fn main() { let x = 1; }"),
            via_new.detect("fn main() { let x = 1; }")
        );
    }
}
