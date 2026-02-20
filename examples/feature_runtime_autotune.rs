//! Example showing runtime host-profile auto-tuning recommendations.

#[cfg(feature = "autotune")]
use http_handle::runtime_autotune::{
    RuntimeTuneRecommendation, detect_host_profile,
};

fn main() {
    #[cfg(feature = "autotune")]
    {
        let profile = detect_host_profile();
        let recommendation =
            RuntimeTuneRecommendation::from_profile(profile);
        println!(
            "profile cores={} mem_mib={} -> inflight={} queue={} sendfile_threshold={}",
            profile.cpu_cores,
            profile.memory_mib,
            recommendation.max_inflight,
            recommendation.max_queue,
            recommendation.sendfile_threshold_bytes
        );
    }
}
