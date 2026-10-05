// Where the chunk rule cuts synthesized speech: Russian and English speech from macOS `say`
// under the recording conditions of the seam matrix, after different lead-ins, is cut with
// the default chunking config. A cut falls in a pause when the same speech before it was
// degraded is silent there. Needs no speech recognition service.

use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::chunking::{next_chunk_start, next_cut};
use voicepuncher::core::application::ChunkingCfg;
use voicepuncher_e2e_test::speech::{synthesize_with_clean, Condition, Lang, SAMPLE_RATE_HZ};

const LEAD_INS_SECS: [f32; 9] = [0.0, 0.35, 0.7, 1.05, 1.4, 1.75, 2.1, 2.45, 2.8];
const MIN_IN_PAUSE_SHARE: f64 = 0.8;
/// Frame length of the pause measurement.
const FRAME_MS: usize = 20;
/// A frame of the clean speech this quiet is silent.
const SILENT_RMS: f32 = 0.003;
/// Silent frames in a row that make a pause of the speech.
const PAUSE_MIN_FRAMES: usize = 5;

#[test]
fn should_cut_most_chunk_seams_of_synthesized_speech_in_a_pause() {
    // Given
    let cfg = TranscriptionCfg::default();
    let chunking = ChunkingCfg {
        chunk_threshold_secs: cfg.chunk_threshold_secs,
        chunk_overlap_secs: cfg.chunk_overlap_secs,
        vad_lookback_secs: cfg.vad_lookback_secs,
    };
    // The phone line filters the speech, which shifts it against the clean copy.
    let conditions = Condition::ALL
        .into_iter()
        .filter(|&c| c != Condition::PhoneBand);
    let cases: Vec<(Lang, Condition)> = Lang::ALL
        .into_iter()
        .flat_map(|lang| conditions.clone().map(move |c| (lang, c)))
        .collect();

    // When
    let results: Vec<(Lang, Condition, usize, usize)> = std::thread::scope(|scope| {
        let handles: Vec<_> = cases
            .iter()
            .map(|&(lang, condition)| {
                scope.spawn(move || {
                    let (mut in_pause, mut seams) = (0, 0);
                    let speeches = synthesize_with_clean(lang, condition, &LEAD_INS_SECS).unwrap();
                    for (speech, clean) in speeches {
                        for cut in cuts(&speech, chunking) {
                            seams += 1;
                            in_pause += usize::from(is_in_pause(&clean, cut));
                        }
                    }
                    (lang, condition, in_pause, seams)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Then
    let (in_pause, seams) = results
        .iter()
        .fold((0, 0), |(p, s), &(_, _, ip, n)| (p + ip, s + n));
    for (lang, condition, ip, n) in &results {
        eprintln!("  {lang:?} {condition:<16?} {ip}/{n} seams in a pause");
    }
    let share = in_pause as f64 / seams as f64;
    eprintln!("  total: {in_pause}/{seams} = {:.0} %", share * 100.0);
    assert!(
        seams > 0,
        "no chunk seams: the speech is shorter than the threshold"
    );
    assert!(
        share >= MIN_IN_PAUSE_SHARE,
        "only {in_pause} of {seams} chunk seams ({:.0} %) fall in a pause of the speech",
        share * 100.0
    );
}

/// Every cut the chunk rule makes in `samples`, the way a recording file is split.
fn cuts(samples: &[i16], cfg: ChunkingCfg) -> Vec<usize> {
    let chunk_len = cfg.chunk_threshold_secs as usize * SAMPLE_RATE_HZ as usize;
    let overlap = cfg.chunk_overlap_secs as usize * SAMPLE_RATE_HZ as usize;
    let mut cuts = Vec::new();
    let mut start = 0;
    while samples.len() - start > chunk_len {
        let cut = next_cut(samples, SAMPLE_RATE_HZ, start, cfg);
        cuts.push(cut);
        start = next_chunk_start(start, cut, overlap);
    }
    cuts
}

/// Whether `cut` lies in a run of at least [`PAUSE_MIN_FRAMES`] silent frames of `clean`.
fn is_in_pause(clean: &[i16], cut: usize) -> bool {
    let frame = SAMPLE_RATE_HZ as usize * FRAME_MS / 1000;
    let silent = |i: usize| {
        let samples = &clean[i * frame..((i + 1) * frame).min(clean.len())];
        let mean_square = samples
            .iter()
            .map(|&s| (s as f32 / 32_768.0).powi(2))
            .sum::<f32>()
            / samples.len().max(1) as f32;
        !samples.is_empty() && mean_square.sqrt() < SILENT_RMS
    };
    let frames = clean.len().div_ceil(frame);
    let at = cut / frame;
    if at >= frames || !silent(at) {
        return false;
    }
    let first = (0..at).rev().take_while(|&i| silent(i)).count();
    let last = (at + 1..frames).take_while(|&i| silent(i)).count();
    first + 1 + last >= PAUSE_MIN_FRAMES
}
