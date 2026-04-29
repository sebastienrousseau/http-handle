// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Run every http-handle example in sequence and report pass/fail.
//!
//! Usage: `cargo run --example all`
//!
//! Examples gated on a Cargo feature are launched with that feature
//! enabled. The bench / dhat examples spawn a long-running server and
//! are skipped here on purpose — they're driven by `scripts/load_test.sh`.

use std::process::{Command, Stdio};
use std::time::Instant;

struct Demo {
    name: &'static str,
    features: Option<&'static str>,
}

const DEMOS: &[Demo] = &[
    // Core
    Demo {
        name: "hello",
        features: None,
    },
    Demo {
        name: "builder",
        features: None,
    },
    Demo {
        name: "request",
        features: None,
    },
    Demo {
        name: "response",
        features: None,
    },
    Demo {
        name: "errors",
        features: None,
    },
    Demo {
        name: "policies",
        features: None,
    },
    Demo {
        name: "pool",
        features: None,
    },
    Demo {
        name: "shutdown",
        features: None,
    },
    Demo {
        name: "keepalive",
        features: None,
    },
    Demo {
        name: "language",
        features: None,
    },
    // Per feature
    Demo {
        name: "async",
        features: Some("async"),
    },
    Demo {
        name: "batch",
        features: Some("batch"),
    },
    Demo {
        name: "streaming",
        features: Some("streaming"),
    },
    Demo {
        name: "optimized",
        features: Some("optimized"),
    },
    Demo {
        name: "observability",
        features: Some("observability"),
    },
    Demo {
        name: "http2",
        features: Some("http2"),
    },
    Demo {
        name: "http3",
        features: Some("http3-profile"),
    },
    Demo {
        name: "perf",
        features: Some("high-perf"),
    },
    Demo {
        name: "multi",
        features: Some("high-perf-multi-thread"),
    },
    Demo {
        name: "autotune",
        features: Some("autotune"),
    },
    Demo {
        name: "ratelimit",
        features: Some("distributed-rate-limit"),
    },
    Demo {
        name: "tenant",
        features: Some("multi-tenant"),
    },
    Demo {
        name: "tls",
        features: Some("enterprise"),
    },
    Demo {
        name: "auth",
        features: Some("enterprise"),
    },
    Demo {
        name: "config",
        features: Some("enterprise"),
    },
    Demo {
        name: "enterprise",
        features: Some("enterprise"),
    },
    // Tooling that exits cleanly. `bench` and `dhat` are excluded
    // intentionally — `bench` runs an indefinite accept loop, and
    // `dhat` writes a profile file as a side effect.
    Demo {
        name: "full",
        features: None,
    },
];

fn main() {
    println!("\n  \x1b[1mhttp-handle examples\x1b[0m\n");

    let start = Instant::now();
    let mut passed = 0;
    let mut failed = 0;

    for demo in DEMOS {
        let label = match demo.features {
            Some(f) => format!("{} (--features {f})", demo.name),
            None => demo.name.to_string(),
        };
        print!("  \x1b[90m{label:<48}\x1b[0m");

        let mut cmd = Command::new("cargo");
        let _ = cmd
            .args(["run", "--example", demo.name, "--quiet"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(features) = demo.features {
            let _ = cmd.args(["--features", features]);
        }
        let result = cmd.status();
        match result {
            Ok(status) if status.success() => {
                println!("\x1b[32mdone\x1b[0m");
                passed += 1;
            }
            _ => {
                println!("\x1b[31mfail\x1b[0m");
                failed += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    println!();
    if failed == 0 {
        println!(
            "  \x1b[1;32m{passed} examples passed\x1b[0m \x1b[90m({:.1}s)\x1b[0m\n",
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "  \x1b[1;31m{failed} failed\x1b[0m, {passed} passed \x1b[90m({:.1}s)\x1b[0m\n",
            elapsed.as_secs_f64()
        );
        std::process::exit(1);
    }
}
