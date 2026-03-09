# Documentation Quality Rubric (Hard Gate)

This rubric is enforced in CI and must always score **100/100**.

## Scoring Model

Each category is worth 10 points:

1. README includes `Architectural Overview`.
2. README includes `Feature List`.
3. README includes `Quick Start`.
4. README includes `Platform Support Matrix`.
5. Crate root uses `#![doc = include_str!("../README.md")]`.
6. Crate root includes rustdoc branding metadata (`html_favicon_url`, `html_logo_url`, `html_root_url`).
7. Crate root enables docs.rs cfg support (`cfg_attr(docsrs, feature(doc_cfg))`).
8. `Cargo.toml` docs.rs metadata enables `all-features` and `rustdoc-args = ["--cfg", "docsrs"]`.
9. `Cargo.toml` docs.rs metadata declares both macOS and Linux targets.
10. Feature-gated modules in `src/lib.rs` expose `doc(cfg(feature = "..."))`.
11. Narrative tutorials exist and are linked from `README.md`.
12. Architecture diagrams exist and are linked from `README.md`.
13. Benchmark reproducibility guide exists and is linked from `README.md`.
14. Error causes and recovery guide exists and is linked from `README.md`.
15. Deprecation/migration policy exists and is linked from `README.md`.
16. Tutorials include an explicit error-recovery and deprecation-readiness section.
17. LTS and lifecycle policy exists and is linked from `README.md`.
18. Migration guide exists and is linked from `README.md`.
19. Recipes guide exists and is linked from `README.md`.
20. Security findings SLA policy exists and is linked from `README.md`.

## Pass/Fail

- **Pass**: 100/100
- **Fail**: Anything below 100/100

## Notes

- This is a release-quality baseline gate.
- API-surface section completeness is additionally enforced by `scripts/score_docs_api_surface.sh`.
