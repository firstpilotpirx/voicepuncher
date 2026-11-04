//! The gate after a long dictation: the E2E cues scenario "Restart said on its own gives its
//! cue at once" fed as the app sends it, so the pause after the lone word is heard at once.

use super::*;

/// The pause after the lone word the gate may still call speech: a fresh gate takes about
/// 0.13 s, a gate that kept the state of 20 s of sentences 0.43 s.
const HANGOVER_LIMIT_SECS: f32 = 0.25;
/// 20 ms: the frames the app sends.
const APP_FRAME: usize = SAMPLE_RATE / 50;

fn with_pauses(voice: Option<&str>, words: &str) -> Vec<f32> {
    let pause = vec![0.0f32; SAMPLE_RATE];
    [pause.as_slice(), &common::spoken_by(voice, words), &pause].concat()
}

/// A voice start, 20 s of dictated sentences and `said` alone after a pause, cut after its
/// last sound, then 2 s of silence; with the sample its words end at.
fn long_dictation_then(said: &str) -> (Vec<f32>, usize) {
    let mut audio = with_pauses(None, "start");
    let sentences = [
        "Weather is sunny in Berlin today",
        "Please bring the slides to the review on Monday",
        "The train leaves at seven and arrives before noon",
    ];
    let mut dictated = Vec::new();
    for sentence in sentences.iter().cycle() {
        if dictated.len() >= 20 * SAMPLE_RATE {
            break;
        }
        dictated.extend(with_pauses(Some("Eddy"), sentence));
    }
    audio.extend(dictated);
    audio.extend(vec![0.0f32; SAMPLE_RATE]);
    let spoken = common::spoken_by(None, said);
    let last_sound = spoken
        .iter()
        .rposition(|s| s.abs() > 0.01)
        .map_or(0, |at| at + 1);
    audio.extend(&spoken[..last_sound]);
    let words_end = audio.len();
    audio.extend(vec![0.0f32; 2 * SAMPLE_RATE]);
    (audio, words_end)
}

/// The stream times the gate heard speech end at, `audio` fed frame by frame.
fn speech_ends(engine: &StreamingEngine, audio: &[f32]) -> Vec<f32> {
    let mut session = engine.session_for(1).expect("a session");
    audio
        .chunks(APP_FRAME)
        .flat_map(|frame| session.accept(frame))
        .filter_map(|event| match event {
            ServerEvent::SpeechEnd { t } => Some(t),
            _ => None,
        })
        .collect()
}

mod needs_streaming_models {
    use super::*;

    #[test]
    fn should_hear_the_pause_after_a_lone_restart_at_once_after_a_long_dictation() {
        let engine = engine();
        // Given a voice start, 20 s of dictated sentences and a lone "restart"
        let (audio, words_end) = long_dictation_then("restart");
        // When
        let ends = speech_ends(&engine, &audio);
        // Then the gate hears the pause after "restart" as soon as after a lone word
        let words_end = words_end as f32 / SAMPLE_RATE as f32;
        let last = ends.last().copied().expect("the speech end of restart");
        assert!(
            last - words_end <= HANGOVER_LIMIT_SECS,
            "speech heard until {last:.2}s, {:.0} ms after the words ended at {words_end:.2}s; ends: {ends:?}",
            (last - words_end) * 1000.0
        );
    }
}
