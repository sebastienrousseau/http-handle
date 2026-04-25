// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2023 - 2026 HTTP Handle

//! Runtime auto-tuning helpers based on detected host resource profile.

use std::num::NonZeroUsize;

/// Host resource profile used for tuning decisions.
///
/// # Examples
///
/// ```rust
/// use http_handle::runtime_autotune::HostResourceProfile;
/// let p = HostResourceProfile { cpu_cores: 4, memory_mib: 4096 };
/// assert_eq!(p.cpu_cores, 4);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostResourceProfile {
    /// Detected logical cores.
    pub cpu_cores: usize,
    /// Estimated memory in MiB.
    pub memory_mib: usize,
}

/// Tuning recommendation independent of server runtime type.
///
/// # Examples
///
/// ```rust
/// use http_handle::runtime_autotune::RuntimeTuneRecommendation;
/// let rec = RuntimeTuneRecommendation { max_inflight: 128, max_queue: 512, sendfile_threshold_bytes: 65536 };
/// assert_eq!(rec.max_queue, 512);
/// ```
///
/// # Panics
///
/// This type does not panic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeTuneRecommendation {
    /// Max concurrent inflight requests.
    pub max_inflight: usize,
    /// Max queued requests.
    pub max_queue: usize,
    /// Threshold for sendfile fast-path.
    pub sendfile_threshold_bytes: u64,
}

impl RuntimeTuneRecommendation {
    /// Produces recommendation from host profile.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::runtime_autotune::{HostResourceProfile, RuntimeTuneRecommendation};
    /// let rec = RuntimeTuneRecommendation::from_profile(HostResourceProfile { cpu_cores: 8, memory_mib: 8192 });
    /// assert!(rec.max_inflight >= 64);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn from_profile(profile: HostResourceProfile) -> Self {
        let cores = profile.cpu_cores.max(1);
        let mem = profile.memory_mib.max(256);
        let max_inflight = (cores * 128).clamp(64, 4096);
        let max_queue = (cores * 512).clamp(256, 16384);
        let sendfile_threshold_bytes =
            if mem < 1024 { 256 * 1024 } else { 64 * 1024 };
        Self {
            max_inflight,
            max_queue,
            sendfile_threshold_bytes,
        }
    }
}

#[cfg(feature = "high-perf")]
impl RuntimeTuneRecommendation {
    /// Converts recommendation into high-performance server limits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::runtime_autotune::RuntimeTuneRecommendation;
    /// let rec = RuntimeTuneRecommendation { max_inflight: 1, max_queue: 2, sendfile_threshold_bytes: 3 };
    /// #[cfg(feature = "high-perf")]
    /// {
    ///     let limits = rec.into_perf_limits();
    ///     assert_eq!(limits.max_queue, 2);
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn into_perf_limits(self) -> crate::perf_server::PerfLimits {
        crate::perf_server::PerfLimits {
            max_inflight: self.max_inflight,
            max_queue: self.max_queue,
            sendfile_threshold_bytes: self.sendfile_threshold_bytes,
        }
    }
}

/// Detects host profile from runtime and lightweight OS hints.
///
/// # Examples
///
/// ```rust
/// use http_handle::runtime_autotune::detect_host_profile;
/// let p = detect_host_profile();
/// assert!(p.cpu_cores >= 1);
/// ```
///
/// # Panics
///
/// This function does not panic.
pub fn detect_host_profile() -> HostResourceProfile {
    let cpu_cores = std::thread::available_parallelism()
        .unwrap_or_else(|_| NonZeroUsize::new(1).expect("non-zero"))
        .get();
    let memory_mib = detect_memory_mib().unwrap_or(2048);
    HostResourceProfile {
        cpu_cores,
        memory_mib,
    }
}

fn detect_memory_mib() -> Option<usize> {
    if let Ok(val) = std::env::var("HTTP_HANDLE_MEMORY_MIB")
        && let Ok(parsed) = val.parse::<usize>()
    {
        return Some(parsed);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo")
            && let Some(line) =
                meminfo.lines().find(|l| l.starts_with("MemTotal:"))
        {
            let kb = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<usize>().ok())?;
            return Some(kb / 1024);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn recommendation_scales_with_profile() {
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
        assert!(large.max_inflight > small.max_inflight);
        assert!(large.max_queue > small.max_queue);
        assert!(
            small.sendfile_threshold_bytes
                > large.sendfile_threshold_bytes
        );
    }

    #[test]
    fn detect_profile_has_sane_minimums() {
        let profile = detect_host_profile();
        assert!(profile.cpu_cores >= 1);
        assert!(profile.memory_mib >= 1);
    }

    #[test]
    fn detect_memory_uses_env_hint_when_valid() {
        let _guard = env_lock().lock().expect("env lock");
        let previous = std::env::var("HTTP_HANDLE_MEMORY_MIB").ok();
        // Safety: test-only process env mutation in a bounded scope.
        unsafe { std::env::set_var("HTTP_HANDLE_MEMORY_MIB", "3072") };
        let got = detect_memory_mib();
        if let Some(old) = previous {
            // Safety: restoring process env key snapshot.
            unsafe { std::env::set_var("HTTP_HANDLE_MEMORY_MIB", old) };
        } else {
            // Safety: paired cleanup for key introduced in this test.
            unsafe { std::env::remove_var("HTTP_HANDLE_MEMORY_MIB") };
        }
        assert_eq!(got, Some(3072));
    }

    #[test]
    fn detect_memory_ignores_invalid_env_hint() {
        let _guard = env_lock().lock().expect("env lock");
        let previous = std::env::var("HTTP_HANDLE_MEMORY_MIB").ok();
        // Safety: test-only process env mutation in a bounded scope.
        unsafe {
            std::env::set_var("HTTP_HANDLE_MEMORY_MIB", "not-a-number")
        };
        let got = detect_memory_mib();
        if let Some(old) = previous {
            // Safety: restoring process env key snapshot.
            unsafe { std::env::set_var("HTTP_HANDLE_MEMORY_MIB", old) };
        } else {
            // Safety: paired cleanup for key introduced in this test.
            unsafe { std::env::remove_var("HTTP_HANDLE_MEMORY_MIB") };
        }
        assert!(got.is_none() || got.expect("value") >= 1);
    }

    #[cfg(feature = "high-perf")]
    #[test]
    fn recommendation_maps_to_perf_limits() {
        let rec = RuntimeTuneRecommendation {
            max_inflight: 123,
            max_queue: 456,
            sendfile_threshold_bytes: 789,
        };
        let limits = rec.into_perf_limits();
        assert_eq!(limits.max_inflight, 123);
        assert_eq!(limits.max_queue, 456);
        assert_eq!(limits.sendfile_threshold_bytes, 789);
    }
}
