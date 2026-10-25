//! How long confirmations of Russian speech take: 2.5 s windows of synthesized Russian speech,
//! every 0.5 s, sent with exactly the command spotting service's confirmation request to an
//! idle speech recognition service. A measurement run on demand, not a gate:
//! `VOICEPUNCHER_TRANSCRIPTION_URL=http://127.0.0.1:<port>/v1 cargo test -p voicepuncher-e2e-test
//! --test confirmation_time -- --ignored --nocapture`, under the machine lock.

use std::time::{Duration, Instant};

use voicepuncher_command_spotting::confirm::{Hear, UtteranceTranscriber};
use voicepuncher_e2e_test::conversions::saturating_usize;
use voicepuncher_e2e_test::speech::Lang;
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

const SAMPLE_RATE_HZ: usize = 16_000;
const WINDOW_SECS: f32 = 2.5;
const STEP_SECS: f32 = 0.5;
/// Russian speech this long: as long as the speech the bar was first measured on.
const SPEECH_SECS: usize = 85;
const DEADLINE: Duration = Duration::from_secs(30);
/// The bar of `limit-confirmation-fallback`: every confirmation within 2 s on an idle service.
const MAX_CONFIRMATION: Duration = Duration::from_secs(2);

#[test]
#[ignore = "a measurement on a running speech recognition service"]
fn confirmations_of_russian_speech_stay_under_two_seconds() {
    let url = std::env::var("VOICEPUNCHER_TRANSCRIPTION_URL")
        .expect("VOICEPUNCHER_TRANSCRIPTION_URL names the speech recognition service");
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    let speech = russian_speech();
    let window = saturating_usize(WINDOW_SECS * SAMPLE_RATE_HZ as f32);
    let step = saturating_usize(STEP_SECS * SAMPLE_RATE_HZ as f32);
    let mut times: Vec<Duration> = (0..=speech.len() - window)
        .step_by(step)
        .map(|start| {
            let started = Instant::now();
            transcriber
                .hear(&speech[start..start + window], DEADLINE)
                .expect("confirmation");
            started.elapsed()
        })
        .collect();
    times.sort();
    let at = |share: f64| times[saturating_usize(((times.len() - 1) as f64 * share).round())];
    let worst = *times.last().expect("windows");
    eprintln!(
        "{} windows: p50 {:.3} s, p95 {:.3} s, max {:.3} s, over 2 s: {}",
        times.len(),
        at(0.5).as_secs_f64(),
        at(0.95).as_secs_f64(),
        worst.as_secs_f64(),
        times.iter().filter(|t| **t > MAX_CONFIRMATION).count()
    );
    assert!(
        worst < MAX_CONFIRMATION,
        "the slowest confirmation took {worst:?}"
    );
}

/// [`SPEECH_SECS`] of Russian speech: the seam test's text, repeated as needed.
fn russian_speech() -> Vec<f32> {
    let text = synthesized(&SayRequest {
        voice: Some("Milena"),
        words_per_minute: None,
        text: Lang::Ru.text(),
    })
    .expect("synthesized Russian speech");
    text.iter()
        .cycle()
        .take(SPEECH_SECS * SAMPLE_RATE_HZ)
        .map(|s| f32::from(*s) / f32::from(i16::MAX))
        .collect()
}
