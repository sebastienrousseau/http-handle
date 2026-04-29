// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! `autotune` feature: derive `PerfLimits` from detected host profile.
//!
//! Run: `cargo run --example autotune --features autotune`

#[cfg(feature = "autotune")]
#[path = "support.rs"]
mod support;

#[cfg(feature = "autotune")]
fn main() {
    use http_handle::runtime_autotune::{
        HostResourceProfile, RuntimeTuneRecommendation,
        detect_host_profile,
    };

    support::header("http-handle -- autotune");

    support::task_with_output(
        "Detect host profile from /proc/meminfo + available_parallelism",
        || {
            let profile = detect_host_profile();
            vec![
                format!("cpu_cores  = {}", profile.cpu_cores),
                format!("memory_mib = {}", profile.memory_mib),
            ]
        },
    );

    support::task_with_output(
        "Recommend PerfLimits for the detected host",
        || {
            let recommendation =
                RuntimeTuneRecommendation::from_profile(
                    detect_host_profile(),
                );
            vec![
                format!(
                    "max_inflight             = {}",
                    recommendation.max_inflight
                ),
                format!(
                    "max_queue                = {}",
                    recommendation.max_queue
                ),
                format!(
                    "sendfile_threshold_bytes = {}",
                    recommendation.sendfile_threshold_bytes
                ),
            ]
        },
    );

    support::task_with_output(
        "Recommendation scales with the profile",
        || {
            let small = RuntimeTuneRecommendation::from_profile(
                HostResourceProfile {
                    cpu_cores: 2,
                    memory_mib: 512,
                },
            );
            let large = RuntimeTuneRecommendation::from_profile(
                HostResourceProfile {
                    cpu_cores: 16,
                    memory_mib: 16384,
                },
            );
            vec![
                format!("small (2 / 512MiB)   = {small:?}"),
                format!("large (16 / 16GiB)   = {large:?}"),
            ]
        },
    );

    support::summary(3);
}

#[cfg(not(feature = "autotune"))]
fn main() {
    eprintln!(
        "Enable the 'autotune' feature: cargo run --example autotune --features autotune"
    );
}
