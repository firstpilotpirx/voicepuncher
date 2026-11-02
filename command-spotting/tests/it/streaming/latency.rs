//! How soon after its words a command is reported, in wall-clock time, when the audio comes in
//! real time as from a microphone.

use std::time::{Duration, Instant};

use voicepuncher_command_spotting::protocol::{PhraseSelection, ServerEvent};
use voicepuncher_command_spotting::streaming::engine::StreamingEngine;

use super::{SAMPLE_RATE, PHRASES};

/// 20 ms: a real microphone delivers audio about this often.
const MICROPHONE_FRAME: usize = SAMPLE_RATE / 50;
/// The bound of a doubled form's start cue (voice-control spec).
pub const DOUBLED_BOUND: Duration = Duration::from_millis(500);

/// Audio said in real time: `before`, then a "start" command, then `after`; how long after the
/// last sample of the command its report came, if it came within 3 s.
pub fn report_delay(
    engine: &StreamingEngine,
    before: &[f32],
    command: &[f32],
    after: &[f32],
) -> Option<Duration> {
    let mut session = engine.session_for(1).expect("a session");
    let phrases = PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session
        .select(&PhraseSelection { phrases, ..PhraseSelection::default() })
        .expect("known phrases");
    let words_end = before.len() + command.len();
    let tail = vec![0.0; 3 * SAMPLE_RATE];
    let audio = [before, command, after, &tail].concat();
    let began = Instant::now();
    let mut words_ended = None;
    for (n, frame) in audio.chunks(MICROPHONE_FRAME).enumerate() {
        let due = began + Duration::from_millis(20) * u32::try_from(n).expect("few frames");
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let mut events = session.accept(frame);
        events.extend(session.poll());
        let fed = (n + 1) * MICROPHONE_FRAME;
        if fed >= words_end && words_ended.is_none() {
            words_ended = Some(Instant::now());
        }
        let reported = events.iter().any(|e| {
            matches!(e, ServerEvent::Command { commands, .. } if commands.iter().any(|c| c == "start dictation"))
        });
        if reported {
            return Some(words_ended.map_or(Duration::ZERO, |ended| ended.elapsed()));
        }
    }
    None
}
