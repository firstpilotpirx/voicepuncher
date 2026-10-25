//! Stand-in hearings: starved, steady after the first, noted and finely timed.

use super::*;

/// The chain said without pauses in a dictation, and the commands it carries out in order.
pub(super) const CHAIN_SENTENCE: &str =
    "We moved the weekly meeting stop enter start and the notes begin here";

pub(super) const CHAIN: [&str; 3] = ["stop dictation", "press enter", "start dictation"];

/// Asserts the chain is carried out when each confirmation takes `hearing`. One test per speed,
/// so nextest runs them in processes of their own at the same time.
pub(super) fn assert_chain_carried_out(hearing: Duration) {
    let listening: Vec<&str> = DICTATING.into_iter().chain(["start dictation"]).collect();
    let events = heard_in_time(&listening, CHAIN_SENTENCE, hearing);
    assert_eq!(
        carried_out(&events),
        CHAIN,
        "hearing in {hearing:?}: {events:?}"
    );
}

/// How long a busy machine keeps the threads that hear waiting for the processor.
pub(super) const STARVED_FOR: Duration = Duration::from_millis(900);

/// A hearing on a busy machine: `inner` hears only after waiting `STARVED_FOR`.
pub(super) struct Starved<H>(pub(super) H);

impl<H: Hear> Hear for Starved<H> {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        std::thread::sleep(STARVED_FOR);
        self.0.hear(clip, deadline)
    }
}

/// Every command reported, alone or placed, in order.
pub(super) use crate::engine::reported_phrases as said;

/// A hearing that answers each clip of a segment `first` and its steady clip `steady`, each
/// `delay` after it is asked in the speech, and counts the steady clips.
pub(super) struct HearsFirstThenSteady {
    pub(crate) first: &'static str,
    pub(crate) steady: &'static str,
    pub(crate) delay: Duration,
    pub(crate) steady_clips: Mutex<usize>,
}

/// The steady clip starts 0.25 s before its first sound; every other clip starts with the
/// pre-roll of 0.8 s before the detected speech.
pub(super) const STEADY_LEAD_MAX_SECS: f32 = 0.45;

impl Hear for HearsFirstThenSteady {
    fn hear(&self, clip: &[f32], _deadline: Duration) -> anyhow::Result<Heard> {
        let onset = clip
            .iter()
            .position(|s| s.abs() > SILENCE_LEVEL)
            .unwrap_or(0);
        let onset_secs = onset as f32 / SAMPLE_RATE as f32;
        let text = if onset_secs < STEADY_LEAD_MAX_SECS {
            *self.steady_clips.lock().unwrap() += 1;
            self.steady
        } else {
            self.first
        };
        Ok(Heard {
            text: format!(" {text}"),
            words: vec![TimedWord {
                text: format!(" {text}"),
                start: onset_secs,
                end: clip.len() as f32 / SAMPLE_RATE as f32,
            }],
        })
    }
}

/// The events of "restart" said alone in a dictation, heard by `hearing`, and how long after
/// the end of the word the first phrase came out.
pub(super) fn restart_alone_heard_by(
    hearing: Arc<HearsFirstThenSteady>,
) -> (Vec<ServerEvent>, f32) {
    let cfg = engine_cfg(None);
    // The default voice, as the keyword spotter hears its "restart".
    let word = sound_only(&common::spoken_by(None, "Restart"));
    let mut stream = silence(PAUSE_SECS);
    stream.extend(&word);
    let said_end = stream.len();
    // Long enough for two confirmations one after another to come back while it is fed.
    stream.extend(silence(3.0));
    let delay = hearing.delay;
    let engine = clocked_engine(&cfg, hearing, delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&DICTATING)).unwrap();
    let mut first_phrase_at = None;
    let events = fed_as_spoken_while(&mut session, &engine.clock, &stream, |_, fed, events| {
        if first_phrase_at.is_none() && !phrases(events).is_empty() {
            first_phrase_at = Some(fed);
        }
    });
    eprintln!("{events:?}");
    let after = first_phrase_at.map_or(f32::MAX, |at| {
        at.saturating_sub(said_end) as f32 / SAMPLE_RATE as f32
    });
    (events, after)
}

/// 10 ms: frames this short time an event to within 10 ms.
pub(super) const FINE_FRAME_SAMPLES: usize = 160;

/// A scripted hearing that notes the sample of the speech at which each clip is asked, and
/// hears a word wrong in every clip asked before a sample.
pub(super) struct NotedHearing {
    pub(crate) hearing: ScriptedHearing,
    pub(crate) clock: Arc<StreamClock>,
    pub(crate) asked: Mutex<Vec<usize>>,
    /// A word and what it is heard as, in clips asked before `heard_right_from`.
    pub(crate) mishears: Option<(&'static str, &'static str)>,
    pub(crate) heard_right_from: usize,
}

impl Hear for NotedHearing {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        let asked = self.clock.now();
        self.asked.lock().unwrap().push(asked);
        let mut heard = self.hearing.hear(clip, deadline)?;
        if let Some((word, wrong)) = self.mishears.filter(|_| asked < self.heard_right_from) {
            for timed in &mut heard.words {
                timed.text = timed.text.replace(word, wrong);
            }
            heard.text = heard.text.replace(word, wrong);
        }
        Ok(heard)
    }
}

/// What a stream of words fed in 10 ms frames brought.
pub(super) struct FinelyHeard {
    pub(crate) events: Vec<ServerEvent>,
    /// How long after the last word ended the first command came out.
    pub(crate) said_after: Option<f32>,
    /// How long after the last word ended each clip was asked, in seconds; negative before.
    pub(crate) asked_after: Vec<f32>,
    /// How long the last word lasted, in seconds.
    pub(crate) last_word_secs: f32,
    /// Where the last word ended in the stream, in seconds.
    pub(crate) said_end_secs: f32,
}

/// Every event for `words`, each said by `say` after the gap of seconds given with it and fed
/// in 10 ms frames as it is spoken, while the service listens for `phrases` and each
/// confirmation comes back `delay` later in the speech.
pub(super) fn heard_finely(
    phrases: &[&str],
    words: &[(f32, &str)],
    delay: Duration,
) -> FinelyHeard {
    heard_finely_misheard(phrases, words, delay, None)
}

/// A pause, then `words`, each said after the gap of seconds given with it; with where each
/// word was said.
fn scripted_stream(words: &[(f32, &str)]) -> (Vec<f32>, Vec<ScriptedWord>) {
    let mut stream = silence(PAUSE_SECS);
    let mut script = Vec::new();
    for (gap, word) in words {
        stream.extend(silence(*gap));
        let said = sound_only(&speech(word));
        script.push(ScriptedWord {
            text: (*word).to_owned(),
            said: stream.len()..stream.len() + said.len(),
        });
        stream.extend(said);
    }
    (stream, script)
}

/// Every event for `stream` fed in fine frames, with how much of it was fed when the first
/// command was said.
fn fed_finely(
    session: &mut Session<'_>,
    clock: &StreamClock,
    stream: &[f32],
) -> (Vec<ServerEvent>, Option<usize>) {
    let mut said_at = None;
    let events = fed_in_frames(
        session,
        clock,
        stream,
        FINE_FRAME_SAMPLES,
        |_, fed, events| {
            if said_at.is_none() && !said(events).is_empty() {
                said_at = Some(fed);
            }
        },
    );
    (events, said_at)
}

/// [`heard_finely`], with the clips asked before the pause after the last word closes hearing
/// the word `mishears` names as the other one.
pub(super) fn heard_finely_misheard(
    phrases: &[&str],
    words: &[(f32, &str)],
    delay: Duration,
    mishears: Option<(&'static str, &'static str)>,
) -> FinelyHeard {
    let cfg = engine_cfg(None);
    let (mut stream, script) = scripted_stream(words);
    let said_end = stream.len();
    let last_word_secs = script
        .last()
        .map_or(0.0, |w| w.said.len() as f32 / SAMPLE_RATE as f32);
    // Long enough for the confirmations to come back while it is fed.
    stream.extend(silence(PAUSE_SECS + 2.0 * delay.as_secs_f32()));
    let clock = Arc::new(StreamClock::default());
    let hearing = Arc::new(NotedHearing {
        hearing: ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false },
        clock: Arc::clone(&clock),
        asked: Mutex::new(Vec::new()),
        mishears,
        heard_right_from: said_end + samples(Duration::from_secs_f32(MIN_SILENCE_SECS)),
    });
    let engine = clocked_engine_on(
        clock,
        &cfg,
        Arc::clone(&hearing) as Arc<dyn Hear>,
        delay,
        None,
    );
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(phrases)).unwrap();
    let secs_after_end = |at: usize| (at as f32 - said_end as f32) / SAMPLE_RATE as f32;
    let (events, said_at) = fed_finely(&mut session, &engine.clock, &stream);
    eprintln!("{events:?}");
    let asked = hearing.asked.lock().unwrap();
    let asked_after = asked.iter().map(|&asked| secs_after_end(asked)).collect();
    FinelyHeard {
        events,
        said_after: said_at.map(secs_after_end),
        asked_after,
        last_word_secs,
        said_end_secs: said_end as f32 / SAMPLE_RATE as f32,
    }
}
