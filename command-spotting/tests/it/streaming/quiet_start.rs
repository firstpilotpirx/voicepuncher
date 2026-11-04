//! The user's quiet "старт" the voice gate stopped hearing before its last sound, as the
//! command history recorded it: the recognizers must still decode it at the close of the
//! utterance.

use voicepuncher_command_spotting::protocol::{PhraseSelection, ServerEvent};
use voicepuncher_command_spotting::streaming::ports::SpeechGate;
use voicepuncher_test_steps::required_env;

use super::{commands_of, engine, SAMPLE_RATE, STANDBY};
use crate::common;

/// 10 ms: the frames the app sends.
const FRAME: usize = SAMPLE_RATE / 100;
/// The audio the service replays into the recognizers ahead of the gate's onset.
const REPLAY_SECS: f32 = 0.5;
/// Audio fed after the gate's last speech: more than the silence that closes an utterance.
const AFTER_SECS: f32 = 2.0;

/// The quiet starts of session 6, each from a command history entry: where the word begins
/// in the recording (the `start` of its row) and for how long the gate heard it at the time,
/// before the word's last sound (the entry's `speech_end` less its onset).
const QUIET_STARTS: [(f32, f32); 2] = [(13.2, 0.47), (17.58, 0.555)];

/// Hears speech in the windows between `from` and `until` samples of the stream, as the
/// service's gate did at the time.
struct Replayed {
    from: usize,
    until: usize,
    seen: usize,
}

impl SpeechGate for Replayed {
    fn hears(&mut self, window: &[f32]) -> bool {
        let begins = self.seen;
        self.seen += window.len();
        begins < self.until && self.seen > self.from
    }
}

fn at(secs: f32) -> usize {
    voicepuncher_command_spotting::conversions::saturating_usize(secs * SAMPLE_RATE as f32)
}

/// The commands reported for the quiet start at `start` of `recording`, the gate hearing it
/// for `heard` seconds, the audio fed in the app's frames.
fn commands_for_quiet_start(recording: &[f32], start: f32, heard: f32) -> Vec<String> {
    let engine = engine();
    let from = at(start - REPLAY_SECS);
    let audio = &recording[from..at(start + heard + AFTER_SECS).min(recording.len())];
    let gate = Replayed { from: at(REPLAY_SECS), until: at(REPLAY_SECS + heard), seen: 0 };
    let mut session = engine.session_with_gate(1, Box::new(gate));
    let phrases = STANDBY.iter().map(|p| (*p).to_owned()).collect();
    session
        .select(&PhraseSelection { phrases, ..PhraseSelection::default() })
        .expect("known phrases");
    let mut events: Vec<ServerEvent> = Vec::new();
    for frame in audio.chunks(FRAME) {
        events.extend(session.accept(frame));
        events.extend(session.poll());
    }
    events.extend(session.sync(0));
    commands_of(events)
}

mod needs_streaming_models {
    use super::*;

    mod needs_voice_samples {
        use super::*;

        #[test]
        fn should_decode_the_users_quiet_start_the_gate_stopped_hearing_early() {
            let root = required_env("VOICEPUNCHER_VOICE_SAMPLES");
            let recording = common::recorded_session(&root, "session-6");
            for (start, heard) in QUIET_STARTS {
                // When
                let commands = commands_for_quiet_start(&recording, start, heard);
                // Then
                assert_eq!(commands, ["start dictation"], "the start at {start} s");
            }
        }
    }
}
