//! How a session finds speech and cuts it into segments.

use super::{
    as_given, raised, samples_in, save_speech, seconds, ClosedSegment, HeadCheck, Running, Segment,
    ServerEvent, Session, FIRST_WINDOW_SECS, PRE_ROLL_SECS, SILERO_WINDOW,
};

impl Session<'_> {
    pub(super) fn detect_speech(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        self.detector.vad_pending.extend_from_slice(samples);
        let windows = self.detector.vad_pending.len() / SILERO_WINDOW;
        for window in self
            .detector
            .vad_pending
            .chunks_exact(SILERO_WINDOW)
            .take(windows)
        {
            self.detector.vad.accept_waveform(&raised(window));
        }
        self.detector.vad_fed += windows * SILERO_WINDOW;
        self.detector.vad_pending.drain(..windows * SILERO_WINDOW);
        let mut events = self.take_segments(None);
        let pause = self
            .detector
            .pause
            .hear(samples, self.detector.vad.detected());
        self.drop_broken_half_pause_hearing();
        if let Some(quiet_from) = pause {
            // WHY: after a quiet word the detector holds speech up to 0.55 s into the pause,
            // so a short pause before more speech would never end the utterance.
            self.detector.vad.flush();
            events.extend(self.take_segments(Some(quiet_from)));
            self.detector.pause.forget();
        }
        if let Some(quiet_from) = self.detector.pause.take_half_pause() {
            self.hear_at_half_pause(quiet_from, self.detector.samples_seen + samples.len());
        }
        events
    }

    /// The events of the segments the detector closed; one closed by a quiet pause ends where
    /// the pause began, at `quiet_from`.
    pub(super) fn take_segments(&mut self, quiet_from: Option<usize>) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        while let Some(segment) = self.detector.vad.front() {
            self.detector.vad.pop();
            let start = self.detector.vad_offset + usize::try_from(segment.start()).unwrap_or(0);
            let detected_end = start + usize::try_from(segment.n()).unwrap_or(0);
            let end = quiet_from.map_or(detected_end, |q| q.clamp(start, detected_end));
            let bounds = Segment { start: seconds(start), end: seconds(end) };
            log::info!(
                "speech {:.2}–{:.2}s closed by {}",
                bounds.start,
                bounds.end,
                closed_by(quiet_from)
            );
            if let Some(dir) = self.engine.save_speech_dir.as_deref() {
                save_speech(dir, bounds.start, &as_given(segment.samples()));
            }
            let segment_events = self.orderer.segment(bounds);
            let quiet_level = self.detector.pause.quiet_level();
            let by_pause = quiet_from.is_some();
            let closed = ClosedSegment { start, end, bounds, quiet_level, by_pause };
            events.extend(self.close_segment(&closed, segment_events));
            self.utterances.last_segment_end = end;
            // WHY: after some utterances the detector's state stops it noticing quiet
            // speech; a reset in the silence that closed the segment loses nothing.
            self.detector.vad.reset();
            self.detector.vad_offset = self.detector.vad_fed;
            self.detector.pause.forget();
        }
        events
    }

    /// The utterance heard now, started at `start` when there is none yet.
    pub(super) fn current_or_new(&mut self, start: usize) -> u64 {
        if let Some(id) = self.utterances.current {
            return id;
        }
        let id = self.new_utterance(start, samples_in(FIRST_WINDOW_SECS), false);
        self.utterances.current = Some(id);
        id
    }

    pub(super) fn new_utterance(
        &mut self,
        start: usize,
        first_window_after: usize,
        head_checked: bool,
    ) -> u64 {
        let id = self.utterances.next_utterance;
        self.utterances.next_utterance += 1;
        let next_window = self
            .detector
            .samples_seen
            .saturating_add(first_window_after);
        self.utterances.running.insert(
            id,
            Running {
                start,
                head: if head_checked {
                    HeadCheck::Checked
                } else {
                    HeadCheck::Unchecked
                },
                next_window,
                reported_until: 0,
                last_window: None,
                jobs: 0,
                closed: false,
                segment_answered: false,
            },
        );
        id
    }

    pub(super) fn mark(&mut self, utterance: u64, change: impl FnOnce(&mut Running)) {
        if let Some(running) = self.utterances.running.get_mut(&utterance) {
            change(running);
        }
    }

    pub(super) fn close_current(&mut self) {
        if let Some(id) = self.utterances.current.take() {
            self.mark(id, |r| r.closed = true);
            self.forget_if_done(id);
        }
    }

    pub(super) fn forget_if_done(&mut self, utterance: u64) {
        if self
            .utterances
            .running
            .get(&utterance)
            .is_some_and(|r| r.closed && r.jobs == 0)
        {
            self.utterances.running.remove(&utterance);
        }
    }

    /// `start` moved back by the pre-roll, never past the end of the utterance before.
    pub(super) fn pre_rolled(&self, start: usize) -> usize {
        start
            .saturating_sub(samples_in(PRE_ROLL_SECS))
            .max(self.utterances.last_segment_end.min(start))
    }
}

/// What closed a segment: the pause watch when it ended at a quiet pause, else the voice
/// detector.
fn closed_by(quiet_from: Option<usize>) -> &'static str {
    if quiet_from.is_some() {
        "pause watch"
    } else {
        "voice detector"
    }
}

#[cfg(test)]
mod tests {
    use super::closed_by;

    #[test]
    fn should_name_the_pause_watch_for_a_segment_closed_at_a_quiet_pause() {
        assert_eq!(closed_by(Some(4_800)), "pause watch");
    }

    #[test]
    fn should_name_the_voice_detector_for_a_segment_it_closed() {
        assert_eq!(closed_by(None), "voice detector");
    }
}
