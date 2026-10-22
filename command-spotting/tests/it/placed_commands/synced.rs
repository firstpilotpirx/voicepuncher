//! A sync asked right after a command said alone: the command is reported before the sync
//! is answered, also when its first hearing was useless and the segment was heard again.

use super::*;

/// A hearing whose clips asked before a sample of the speech come back empty, as clips cut
/// short under load do, and right from then on.
struct EmptyBefore {
    hearing: ScriptedHearing,
    clock: Arc<StreamClock>,
    heard_right_from: usize,
}

impl Hear for EmptyBefore {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        if self.clock.now() < self.heard_right_from {
            return Ok(Heard::default());
        }
        self.hearing.hear(clip, deadline)
    }
}

/// Every event for `utterance` said alone in standby, fed in 10 ms frames, each confirmation
/// `delay` late, heard by what `hearing` makes of its scripted hearing, the clock and where
/// its last word ends; a sync is asked `delay` after the speech ends, while the first
/// hearing of the segment is still out.
fn heard_alone_then_synced(
    utterance: &str,
    delay: Duration,
    hearing: impl FnOnce(ScriptedHearing, Arc<StreamClock>, usize) -> Arc<dyn Hear>,
) -> Option<Vec<ServerEvent>> {
    let cfg = engine_cfg(None)?;
    let (mut stream, script) = scripted(utterance);
    let said_end = script.last().map_or(0, |w| w.said.end);
    // Long enough for the confirmations to come back while it is fed.
    stream.extend(silence(2.0 * delay.as_secs_f32()));
    let clock = Arc::new(StreamClock::default());
    let scripted = ScriptedHearing { stream: stream.clone(), script, completes_cut_words: false };
    let hearing = hearing(scripted, Arc::clone(&clock), said_end);
    let engine = clocked_engine_on(clock, &cfg, hearing, delay, None);
    let mut session = engine.engine.session().unwrap();
    session.listen_for(&owned(&STANDBY)).unwrap();
    let sync_at = said_end + samples(delay);
    let mut events = Vec::new();
    let mut synced = false;
    for (n, frame) in stream.chunks(FINE_FRAME_SAMPLES).enumerate() {
        let fed = n * FINE_FRAME_SAMPLES;
        if !synced && fed >= sync_at {
            synced = true;
            events.extend(session.sync(1));
        }
        events.extend(engine.clock.feed(&mut session, frame, fed));
    }
    engine.clock.release_all();
    events.extend(session.settle());
    eprintln!("{events:?}");
    Some(events)
}

/// Asserts that `events` report `command` and no other, before the sync is answered.
fn assert_said_before_synced(events: &[ServerEvent], command: &str) {
    let said = said(events);
    assert!(
        !said.is_empty() && said.iter().all(|c| c == command),
        "{events:?}"
    );
    let synced = events
        .iter()
        .position(|e| matches!(e, ServerEvent::Synced { .. }));
    let said_at = events
        .iter()
        .position(|e| matches!(e, ServerEvent::Phrase { .. } | ServerEvent::Command { .. }));
    assert!(
        synced.is_some_and(|synced| said_at.is_some_and(|said| said < synced)),
        "the sync was answered before the command: {events:?}"
    );
}

#[test]
fn should_report_a_command_whose_first_hearing_is_empty_under_load_before_answering_a_sync() {
    // Given each confirmation 0.6 s late and every clip asked before the pause after the
    // speech closes heard empty, as clips cut short under load are
    // When "clear text please" is said alone in standby and a sync is asked right after it
    let Some(events) = heard_alone_then_synced(
        "clear text please",
        Duration::from_millis(600),
        |hearing, clock, said_end| {
            Arc::new(EmptyBefore {
                hearing,
                clock,
                heard_right_from: said_end + samples(Duration::from_secs_f32(PAUSE_SECS)),
            })
        },
    ) else {
        return;
    };
    // Then the command is reported, and before the sync is answered
    assert_said_before_synced(&events, "clear all text");
}

#[test]
fn should_report_a_command_misheard_at_half_the_pause_before_answering_a_sync() {
    // Given each confirmation 0.6 s late, and every clip asked before the pause closes hearing
    // "clear" as "near", as a clip cut at half the pause may be heard
    // When "okay, clear" is said in standby and a sync is asked right after it
    let Some(events) = heard_alone_then_synced(
        "okay clear",
        Duration::from_millis(600),
        |hearing, clock, said_end| {
            Arc::new(NotedHearing {
                hearing,
                clock,
                asked: Mutex::new(Vec::new()),
                mishears: Some(("clear", "near")),
                heard_right_from: said_end + samples(Duration::from_secs_f32(MIN_SILENCE_SECS)),
            })
        },
    ) else {
        return;
    };
    // Then the command takes effect, the segment heard whole, before the sync is answered
    assert_said_before_synced(&events, "clear all text");
}
