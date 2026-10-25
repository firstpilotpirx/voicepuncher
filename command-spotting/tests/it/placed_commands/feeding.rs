//! Speech fed to the engine as spoken, in frames or in turns, and what it carried out.

use super::*;
use voicepuncher_command_spotting::conversions::saturating_usize;

/// `words` spoken by `say`, without any pause.
pub(super) fn speech(words: &str) -> Vec<f32> {
    common::spoken_by(Some(VOICE), words)
}

/// `say` speech from its first sound to its last.
pub(super) fn sound_only(audio: &[f32]) -> Vec<f32> {
    let loud = |s: &f32| s.abs() > SILENCE_LEVEL;
    let first = audio.iter().position(loud).unwrap_or(0);
    let last = audio.iter().rposition(loud).map_or(first, |at| at + 1);
    audio[first..last].to_vec()
}

pub(super) fn silence(secs: f32) -> Vec<f32> {
    vec![0.0f32; saturating_usize(secs * SAMPLE_RATE as f32)]
}

/// Every event the service reports for `audio`, with a pause before and after it, while
/// it listens for `phrases`.
pub(super) fn heard(engine: &ClockedEngine, phrases: &[&str], audio: &[f32]) -> Vec<ServerEvent> {
    let mut session = engine.engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    let stream: Vec<f32> = silence(PAUSE_SECS)
        .into_iter()
        .chain(audio.iter().copied())
        .chain(silence(PAUSE_SECS))
        .collect();
    dbg!(fed_as_spoken(&mut session, &engine.clock, &stream))
}

pub(super) fn commands(events: &[ServerEvent]) -> Vec<(Vec<String>, Placement)> {
    events
        .iter()
        .filter_map(|e| match e {
            ServerEvent::Command { commands, placement, .. } => {
                Some((commands.clone(), *placement))
            }
            _ => None,
        })
        .collect()
}

/// Every phrase reported alone, in order.
pub(super) use crate::engine::spotted as phrases;

pub(super) fn command(names: &[&str], placement: Placement) -> (Vec<String>, Placement) {
    (names.iter().map(|n| (*n).to_owned()).collect(), placement)
}

/// `sentence` said word by word by `say`, between pauses, and where each word lies.
pub(super) fn scripted(sentence: &str) -> (Vec<f32>, Vec<ScriptedWord>) {
    scripted_utterances(&[sentence])
}

/// Each of `utterances` said word by word by `say`, a pause before, between and after
/// them, and where each word lies.
pub(super) fn scripted_utterances(utterances: &[&str]) -> (Vec<f32>, Vec<ScriptedWord>) {
    let mut stream = silence(PAUSE_SECS);
    let mut script = Vec::new();
    for word_or_pause in utterances.join(" | ").split_whitespace() {
        if word_or_pause == "|" {
            stream.extend(silence(PAUSE_SECS));
            continue;
        }
        say_word(&mut stream, &mut script, word_or_pause);
    }
    stream.extend(silence(PAUSE_SECS));
    (stream, script)
}

/// `word` said by `say` at the end of `stream`, then a short gap, noted in `script`.
pub(super) fn say_word(stream: &mut Vec<f32>, script: &mut Vec<ScriptedWord>, word: &str) {
    let said = sound_only(&speech(word));
    script.push(ScriptedWord {
        text: word.to_owned(),
        said: stream.len()..stream.len() + said.len(),
    });
    stream.extend(said);
    stream.extend(silence(WORD_GAP_SECS));
}

/// Every event for `sentence`, fed as it is spoken, while the service listens for `phrases`
/// and each confirmation comes back `delay` later in the speech.
pub(super) fn heard_in_time(phrases: &[&str], sentence: &str, delay: Duration) -> Vec<ServerEvent> {
    heard_in_time_by(phrases, sentence, delay, |hearing| Arc::new(hearing))
}

/// As `heard_in_time`, heard by what `hearing` makes of the scripted hearing of the sentence.
pub(super) fn heard_in_time_by(
    phrases: &[&str],
    sentence: &str,
    delay: Duration,
    hearing: impl FnOnce(ScriptedHearing) -> Arc<dyn Hear>,
) -> Vec<ServerEvent> {
    let cfg = engine_cfg(None);
    let (stream, script) = scripted(sentence);
    let scripted = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false };
    let engine = clocked_engine(&cfg, hearing(scripted), delay, None);
    let mut session = engine.engine.session().unwrap();
    let phrases: Vec<String> = phrases.iter().map(|p| (*p).to_owned()).collect();
    session.listen_for(&phrases).unwrap();
    dbg!(fed_as_spoken(&mut session, &engine.clock, &stream))
}

/// Every event the session reports for `audio` fed frame by frame on `clock`, each
/// confirmation coming back where its delay in the speech puts it, however busy the machine.
pub(super) fn fed_as_spoken(
    session: &mut Session<'_>,
    clock: &StreamClock,
    audio: &[f32],
) -> Vec<ServerEvent> {
    fed_as_spoken_while(session, clock, audio, |_, _, _| {})
}

/// As `fed_as_spoken`, with `before_frame` called before each frame with the samples fed
/// so far.
pub(super) fn fed_as_spoken_while(
    session: &mut Session<'_>,
    clock: &StreamClock,
    audio: &[f32],
    before_frame: impl FnMut(&mut Session<'_>, usize, &[ServerEvent]),
) -> Vec<ServerEvent> {
    fed_in_frames(session, clock, audio, FRAME_SAMPLES, before_frame)
}

/// As `fed_as_spoken_while`, in frames of `frame_samples`; once the speech ends, every
/// confirmation still held comes back.
pub(super) fn fed_in_frames(
    session: &mut Session<'_>,
    clock: &StreamClock,
    audio: &[f32],
    frame_samples: usize,
    mut before_frame: impl FnMut(&mut Session<'_>, usize, &[ServerEvent]),
) -> Vec<ServerEvent> {
    let mut events = Vec::new();
    for (n, frame) in audio.chunks(frame_samples).enumerate() {
        let fed = n * frame_samples;
        before_frame(session, fed, &events);
        events.extend(clock.feed(session, frame, fed));
    }
    clock.release_all();
    events.extend(session.settle());
    events
}

/// Every event for `utterances` said in standby, fed as they are spoken, with each
/// confirmation `delay` late in the speech and a word cut off by a clip heard whole as whisper hears it;
/// the session moves to the phrases of a running dictation in the pause after the first
/// utterance, before its confirmation comes back, as a dictation the hotkey starts moves it.
pub(super) fn heard_with_dictation_started_at_first_pause(
    utterances: &[&str],
    delay: Duration,
) -> Vec<ServerEvent> {
    let cfg = engine_cfg(None);
    let (stream, script) = scripted_utterances(utterances);
    let first_words = utterances[0].split_whitespace().count();
    let switch_at =
        script[first_words - 1].said.end + saturating_usize(PAUSE_SECS / 2.0 * SAMPLE_RATE as f32);
    let hearing = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: true };
    let engine = clocked_engine(&cfg, Arc::new(hearing), delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&STANDBY)).unwrap();
    let mut dictating = false;
    dbg!(fed_as_spoken_while(
        &mut session,
        &engine.clock,
        &stream,
        |session, fed, _| {
            if !dictating && fed >= switch_at {
                dictating = true;
                session.listen_for(&owned(&DICTATING)).unwrap();
            }
        }
    ))
}

/// Every event for `utterances` said while dictating, fed as they are spoken, with each
/// confirmation `delay` late in the speech; the session moves to the standby phrases once the stop
/// is reported, as the app does when the stop ends its dictation.
pub(super) fn heard_with_standby_once_stopped(
    utterances: &[&str],
    delay: Duration,
) -> Vec<ServerEvent> {
    let cfg = engine_cfg(None);
    let (mut stream, script) = scripted_utterances(utterances);
    // Silence after the last word, while the late confirmations come back.
    stream.extend(vec![0.0; 2 * samples(delay)]);
    let hearing = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: true };
    let engine = clocked_engine(&cfg, Arc::new(hearing), delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&DICTATING)).unwrap();
    let mut stopped = false;
    dbg!(fed_as_spoken_while(
        &mut session,
        &engine.clock,
        &stream,
        |session, _, events| {
            if !stopped && said(events).iter().any(|c| c == "stop dictation") {
                stopped = true;
                session.listen_for(&owned(&STANDBY)).unwrap();
            }
        },
    ))
}

pub(super) fn owned(phrases: &[&str]) -> Vec<String> {
    phrases.iter().map(|p| (*p).to_owned()).collect()
}

/// How many times `phrase` was carried out in a command said without a pause.
pub(super) fn times_carried_out(events: &[ServerEvent], phrase: &str) -> usize {
    commands(events)
        .iter()
        .flat_map(|(names, _)| names.clone())
        .filter(|name| name == phrase)
        .count()
}

/// The commands carried out, in order, whether said in one chain or in several.
pub(super) fn carried_out(events: &[ServerEvent]) -> Vec<String> {
    commands(events)
        .into_iter()
        .flat_map(|(names, _)| names)
        .collect()
}
