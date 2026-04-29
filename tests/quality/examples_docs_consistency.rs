// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Consistency tests for example declarations and example documentation.

use regex::Regex;
use std::path::Path;

fn extract_examples(manifest: &str) -> Vec<(String, String)> {
    let pattern = Regex::new(
        r#"(?ms)\[\[example\]\]\s+.*?name\s*=\s*"([^"]+)".*?path\s*=\s*"([^"]+)""#,
    )
    .expect("valid regex");

    pattern
        .captures_iter(manifest)
        .map(|caps| (caps[1].to_string(), caps[2].to_string()))
        .collect()
}

#[test]
fn cargo_examples_exist_on_disk() {
    let manifest = include_str!("../../Cargo.toml");
    let examples = extract_examples(manifest);
    assert!(
        !examples.is_empty(),
        "No [[example]] entries were found in Cargo.toml"
    );

    for (name, path) in examples {
        assert!(
            Path::new(&path).exists(),
            "Example `{}` references missing file `{}`",
            name,
            path
        );
    }
}

#[test]
fn examples_docs_covers_all_manifest_examples() {
    let manifest = include_str!("../../Cargo.toml");
    let examples_doc = include_str!("../../docs/EXAMPLES.md");
    let examples = extract_examples(manifest);

    for (name, _) in examples {
        assert!(
            examples_doc.contains(&format!("`{}`", name)),
            "docs/EXAMPLES.md is missing example `{}`",
            name
        );
    }
}

#[test]
fn readme_links_examples_matrix() {
    let readme = include_str!("../../README.md");
    assert!(
        readme.contains("docs/EXAMPLES.md"),
        "README should link to docs/EXAMPLES.md"
    );
}
