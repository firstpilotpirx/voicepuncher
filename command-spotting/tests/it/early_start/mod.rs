//! Whether the start of a running utterance is searched early for a command opening speech
//! follows the app's word (`early_start`), not only the phrases it selects.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use voicepuncher_command_spotting::confirm::{Hear, Heard};
use voicepuncher_command_spotting::engine::Engine;
use voicepuncher_command_spotting::protocol::PhraseSelection;

use crate::common;
use crate::engine::{engine, engine_cfg};

/// The phrases of standby, "start dictation" among them.
const STANDBY_PHRASES: [&str; 3] = ["start dictation", "press enter", "clear all text"];
/// Speech long enough for the early and the full check of its start.
const SENTENCE: &str = "so I was saying that the weather today is really nice";
const FRAME_SAMPLES: usize = 1_600;

/// Hears nothing in every clip, at once, and counts the clips.
struct CountedHearing(AtomicU32);

impl Hear for CountedHearing {
    fn hear(&self, _: &[f32], _: Duration) -> anyhow::Result<Heard> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Heard::default())
    }
}

/// The clips the session asks of the speech recognition service for `SENTENCE` in standby,
/// told `early_start`, each answered before the next frame.
fn clips_heard(early_start: Option<bool>) -> u32 {
    let hear = Arc::new(CountedHearing(AtomicU32::new(0)));
    let engine = Engine::load(&engine_cfg(None))
        .expect("models load")
        .hearing(hear.clone());
    let mut session = engine.session().unwrap();
    let phrases = STANDBY_PHRASES.iter().map(|p| (*p).to_owned()).collect();
    session
        .select(&PhraseSelection { phrases, early_start, ..PhraseSelection::default() })
        .unwrap();
    for frame in common::spoken_by(None, SENTENCE).chunks(FRAME_SAMPLES) {
        session.accept(frame);
        session.settle();
    }
    hear.0.load(Ordering::SeqCst)
}

mod needs_kws_models {
    use super::*;

    #[test]
    fn should_not_search_the_start_early_when_the_app_says_no_early_start() {
        let _shared = engine();
        // When
        let untold = clips_heard(None);
        let searched_early = clips_heard(Some(true));
        let not_early = clips_heard(Some(false));
        // Then the early hearings of the start are left out, and an app that says nothing gets
        // them in standby
        assert!(
            not_early < searched_early,
            "{not_early} clips without, {searched_early} with"
        );
        assert_eq!(untold, searched_early);
    }
}
