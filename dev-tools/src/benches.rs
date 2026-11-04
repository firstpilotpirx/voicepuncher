//! The benches (`bench`, `streaming-bench`): model stages (the e2e lock) that start no service, journaled under
//! their own recipe with `VOICEPUNCHER_BENCH_ARGS`, with no key, pass or reuse; the summary shows their time
//! apart from the checks. Named here, where neither the stage runner nor the journal depends on the other.

pub const BENCHES: [&str; 2] = ["bench", "streaming-bench"];

pub fn is_bench(stage: &str) -> bool {
    BENCHES.contains(&stage)
}
