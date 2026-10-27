//! The keyword streams of a session and what the keyword spotter spots in them.

use sherpa_onnx::{KeywordSpotter, OnlineStream};

use crate::keywords;
use crate::orderer::Spotted;
use crate::protocol::ServerEvent;

use super::spotted::CHECKED_WHEN_SPOTTED;
use super::{phrase_span, samples_in, seconds, Session, SAMPLE_RATE, TRIGGERS_KEPT_SECS};

/// The keyword streams of the selected phrases.
#[derive(Default)]
pub(super) struct Spotting {
    pub(super) phrases: Option<OnlineStream>,
    /// Spots the trigger-only keywords of the selected phrases, apart from `phrases`: one
    /// firing there never resets the spotting of the phrases.
    pub(super) triggers: Option<OnlineStream>,
}

impl Session<'_> {
    pub(super) fn spot(&mut self, samples: &[f32]) -> Vec<ServerEvent> {
        let position = seconds(self.detector.samples_seen + samples.len());
        self.spot_triggers(samples, position);
        let Some(stream) = self.spotting.phrases.as_ref() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for (spotted, tokens) in decoded(&self.engine.spotter, stream, samples, position) {
            self.record_spotted(&spotted, &tokens);
            log::info!(
                "spotted \"{}\" at {:.2}–{:.2}s",
                spotted.phrase,
                spotted.start,
                spotted.end
            );
            if self.worker.is_some() && CHECKED_WHEN_SPOTTED.contains(&spotted.phrase.as_str()) {
                self.jobs.spotted_checks.spotted(samples_in(spotted.end));
            }
            events.extend(self.orderer.phrase(spotted));
        }
        events
    }

    /// Spots the triggers of the selected phrases in `samples`, which bring the audio fed to
    /// `position` seconds, and asks the checks they call for.
    fn spot_triggers(&mut self, samples: &[f32], position: f32) {
        let spotter = &self.engine.spotter;
        if let Some(triggers) = self.spotting.triggers.as_ref() {
            // The spotter adds its own keywords, the phrases, to every stream: only a trigger
            // spotted here asks the check.
            let found = decoded(spotter, triggers, samples, position);
            for (trigger, tokens) in found
                .into_iter()
                .filter(|(t, _)| keywords::is_trigger(&t.phrase))
            {
                self.record_spotted(&trigger, &tokens);
                log::info!(
                    "trigger \"{}\" at {:.2}–{:.2}s",
                    trigger.phrase,
                    trigger.start,
                    trigger.end
                );
                self.jobs.spotted_checks.spotted(samples_in(trigger.end));
                self.hear_misheard_again(&trigger);
                self.hear_pending_again(&trigger);
                self.hearings.heard_triggers.push(trigger);
            }
        }
        self.hearings
            .heard_triggers
            .retain(|t| t.end + TRIGGERS_KEPT_SECS > position);
    }
}

/// The keywords `stream` spots in `samples`, which bring the audio fed to `position`
/// seconds, each with the tokens it was decoded as; the stream starts afresh after each.
fn decoded(
    spotter: &KeywordSpotter,
    stream: &OnlineStream,
    samples: &[f32],
    position: f32,
) -> Vec<(Spotted, String)> {
    stream.accept_waveform(i32::try_from(SAMPLE_RATE).unwrap_or(i32::MAX), samples);
    let mut found = Vec::new();
    while spotter.is_ready(stream) {
        spotter.decode(stream);
        let Some(result) = spotter.get_result(stream).filter(|r| !r.keyword.is_empty()) else {
            continue;
        };
        spotter.reset(stream);
        let (start, end) = phrase_span(position, &result.timestamps);
        let keyword = Spotted { phrase: keywords::phrase_of(&result.keyword), start, end };
        found.push((keyword, result.tokens));
    }
    found
}
