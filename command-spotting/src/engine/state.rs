//! The sub-states a session holds: its confirmations, its second hearings, its utterances and
//! its speech detection.

use std::collections::HashMap;

use sherpa_onnx::VoiceActivityDetector;

use crate::orderer::Spotted;
use crate::outbox::Outbox;

use super::confirming::{AskedJob, EarlyRehears, HalfPauseHearing, Rehear};
use super::pause::PauseWatch;
use super::queued::CheckQueue;
use super::spotted::SpottedChecks;
use super::Running;

/// The confirmations asked of the speech recognition service and the events they make.
#[derive(Default)]
pub(in crate::engine) struct Jobs {
    pub(in crate::engine) next_job: u64,
    /// What each confirmation asked for, by job number, until its answer is applied.
    pub(in crate::engine) asked: HashMap<u64, AskedJob>,
    pub(in crate::engine) outbox: Outbox,
    /// The checks of running utterances waiting their turn behind the one asked.
    pub(in crate::engine) checks: CheckQueue,
    /// The check a spotted "start" or "restart" asks of the running utterance.
    pub(in crate::engine) spotted_checks: SpottedChecks,
}

impl Jobs {
    /// The first check of the phrase spotted ending at `said_end` in `utterance` found no
    /// command in audio up to `heard_to`: the second check follows.
    pub(in crate::engine) fn found_nothing(
        &mut self,
        said_end: usize,
        heard_to: usize,
        utterance: u64,
    ) {
        self.spotted_checks
            .found_nothing(said_end, heard_to, utterance);
    }
}

/// The hearings asked beside or instead of a segment's own.
#[derive(Default)]
pub(in crate::engine) struct Hearings {
    /// The triggers spotted in the last seconds: a short segment holding one is heard again
    /// when its first hearing holds no command.
    pub(in crate::engine) heard_triggers: Vec<Spotted>,
    /// The last short segment whose first hearing held no command, with its job: a trigger
    /// the spotter reports in it only after that answer has it heard again.
    pub(in crate::engine) misheard: Option<(u64, Rehear)>,
    /// The second hearings asked beside a short segment's first one.
    pub(in crate::engine) early: EarlyRehears,
    /// The hearing of a short utterance in standby asked at half its pause; it stands for the
    /// utterance's segment when that same quiet run closes it.
    pub(in crate::engine) half_pause: Option<HalfPauseHearing>,
    /// Segment jobs a hearing at half the pause stands for, with where the segment's own clip
    /// starts: an answer holding no command alone has the segment heard whole instead.
    pub(in crate::engine) stand_ins: HashMap<u64, usize>,
}

/// The utterances with confirmations still to apply, and the one heard now.
#[derive(Default)]
pub(in crate::engine) struct Utterances {
    pub(in crate::engine) running: HashMap<u64, Running>,
    pub(in crate::engine) current: Option<u64>,
    pub(in crate::engine) next_utterance: u64,
    /// Where the last utterance ended: the pre-roll of the next never reaches back past it.
    pub(in crate::engine) last_segment_end: usize,
}

/// The voice activity detector, the pause watch and the stream clock they share.
pub(in crate::engine) struct Detector {
    pub(in crate::engine) vad: VoiceActivityDetector,
    /// Ends an utterance at a quiet pause the detector still holds as speech.
    pub(in crate::engine) pause: PauseWatch,
    pub(in crate::engine) samples_seen: usize,
    pub(in crate::engine) vad_pending: Vec<f32>,
    /// Samples fed to the voice activity detector so far.
    pub(in crate::engine) vad_fed: usize,
    /// Samples fed before its last reset: its segment positions count from there.
    pub(in crate::engine) vad_offset: usize,
}

impl Detector {
    pub(in crate::engine) const fn new(vad: VoiceActivityDetector, pause: PauseWatch) -> Self {
        Self { vad, pause, samples_seen: 0, vad_pending: Vec::new(), vad_fed: 0, vad_offset: 0 }
    }
}
