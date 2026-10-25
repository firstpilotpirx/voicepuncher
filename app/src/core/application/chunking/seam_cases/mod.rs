//! Chunk seams recorded from whisper-server (large-v3-turbo, greedy decoding): the last
//! words of one chunk transcript, the first words of the next, and the merge after review.
//! The speech is the Russian and English test paragraphs of `voicepuncher_e2e_test::speech`,
//! spoken by macOS `say` under recording conditions and cut with the live chunk rule at
//! several lead-ins, so the seams fall on different words.

pub(super) struct SeamCase {
    pub recording: &'static str,
    pub tail: &'static str,
    pub head: &'static str,
    pub merged: &'static str,
}

mod en_eddy_early;
mod en_eddy_late;
mod en_flo_early;
mod en_flo_late;
mod ru_milena_early;
mod ru_milena_late;

/// Every recorded seam.
pub(super) fn all() -> Vec<&'static SeamCase> {
    [
        ru_milena_early::CASES,
        en_eddy_early::CASES,
        ru_milena_late::CASES,
        en_eddy_late::CASES,
        en_flo_early::CASES,
        en_flo_late::CASES,
    ]
    .into_iter()
    .flatten()
    .collect()
}
