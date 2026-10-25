//! How the speech recognition service for commands answers in the scheduling scenarios: the
//! scripted hearing of the command spotting service's tests, served as a loaded copy over
//! HTTP, or wrong in the hearings asked before a set point of the speech.

use voicepuncher::test_doubles::prelude::*;
use voicepuncher_command_spotting::confirm::{Hear, Heard, UtteranceTranscriber};

use crate::scripted_hearing::ScriptedHearing;

#[path = "../../../../../../command-spotting/tests/it/placed_commands/under_load/scripted_copy.rs"]
mod scripted_copy;
#[path = "../../../../../../command-spotting/tests/it/stream_clock/mod.rs"]
pub(super) mod stream_clock;

use stream_clock::{samples, StreamClock};

/// How long a loaded copy decodes each confirmation: past the budget of an idle machine.
const LOADED_DECODING: Duration = Duration::from_millis(1_200);
/// The pause after the speech that closes its segment.
const SEGMENT_PAUSE: Duration = Duration::from_secs(1);
/// The silence that ends an utterance: a hearing asked before it is the early one.
const UTTERANCE_PAUSE: Duration = Duration::from_millis(300);

/// How the service answers the confirmations of a scenario.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Answers {
    /// The words of each clip, at once.
    #[default]
    Right,
    /// By a copy of the service over HTTP that decodes each for 1.2 s, as on a loaded machine.
    Loaded,
    /// Nothing for every hearing asked before the pause after the speech closed its segment.
    EmptyUntilClosed,
    /// "clear" heard as "near" in every hearing asked before the utterance's pause ended.
    MishearsClearEarly,
}

impl Answers {
    /// The hearing of `scripted` for speech whose last word ends at sample `said_end`, its
    /// early hearings told apart on `clock`.
    pub fn hearing(
        self,
        scripted: ScriptedHearing,
        clock: &Arc<StreamClock>,
        said_end: usize,
    ) -> Result<Arc<dyn Hear>> {
        let early = |scripted, pause: Duration, heard_as| Early {
            scripted,
            clock: Arc::clone(clock),
            right_from: said_end + samples(pause),
            heard_as,
        };
        Ok(match self {
            Self::Right => Arc::new(scripted),
            Self::Loaded => {
                let url = scripted_copy::serve(scripted, LOADED_DECODING);
                Arc::new(UtteranceTranscriber::new(&url)?)
            }
            Self::EmptyUntilClosed => Arc::new(early(scripted, SEGMENT_PAUSE, None)),
            Self::MishearsClearEarly => {
                Arc::new(early(scripted, UTTERANCE_PAUSE, Some(("clear", "near"))))
            }
        })
    }
}

/// The scripted hearing, wrong in the hearings asked before `right_from` in the speech:
/// they hear nothing, or one word as another.
struct Early {
    scripted: ScriptedHearing,
    clock: Arc<StreamClock>,
    right_from: usize,
    heard_as: Option<(&'static str, &'static str)>,
}

impl Hear for Early {
    fn hear(&self, clip: &[f32], deadline: Duration) -> Result<Heard> {
        let heard = self.scripted.hear(clip, deadline)?;
        if self.clock.now() >= self.right_from {
            return Ok(heard);
        }
        let Some((word, wrong)) = self.heard_as else {
            return Ok(Heard::default());
        };
        let mut words = heard.words;
        for timed in &mut words {
            timed.text = timed.text.replace(word, wrong);
        }
        Ok(Heard { text: heard.text.replace(word, wrong), words })
    }
}
