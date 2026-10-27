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
    /// The phrases spotted in `samples`, and the doubled commands they complete that go out
    /// at once, without confirmation.
    pub(super) fn spot(&mut self, samples: &[f32]) -> (Vec<ServerEvent>, Vec<ServerEvent>) {
        let position = seconds(self.detector.samples_seen + samples.len());
        let mut at_once = self.spot_triggers(samples, position);
        let Some(stream) = self.spotting.phrases.as_ref() else {
            return (Vec::new(), at_once);
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
            if self.worker.is_some() {
                at_once.extend(self.ask_checks_of(&spotted));
            }
            let late = self.orderer.phrase(spotted);
            let verdict = self.report_late(&late);
            events.extend(late);
            events.extend(verdict);
        }
        (events, at_once)
    }

    /// Notes `spotted` for the checks it asks: of a "start" or "restart", and of a doubled
    /// form, which goes out at once without confirmation.
    fn ask_checks_of(&mut self, spotted: &Spotted) -> Option<ServerEvent> {
        let said_end = samples_in(spotted.end);
        if CHECKED_WHEN_SPOTTED.contains(&spotted.phrase.as_str()) {
            self.jobs.spotted_checks.spotted(said_end);
        }
        if self.jobs.spotted_checks.hit(&spotted.phrase, said_end) {
            return self.report_doubled(spotted);
        }
        None
    }

    /// Spots the triggers of the selected phrases in `samples`, which bring the audio fed to
    /// `position` seconds, and asks the checks they call for; a doubled trigger goes out at
    /// once without confirmation.
    fn spot_triggers(&mut self, samples: &[f32], position: f32) -> Vec<ServerEvent> {
        let mut at_once = Vec::new();
        let spotter = &self.engine.spotter;
        if let Some(triggers) = self.spotting.triggers.as_ref() {
            // The spotter adds its own keywords, the phrases, to every stream: only a trigger
            // spotted here asks the check.
            let found = decoded(spotter, triggers, samples, position);
            for (trigger, tokens) in found
                .into_iter()
                .filter(|(t, _)| keywords::is_trigger(&t.phrase))
            {
                at_once.extend(self.heard_trigger(trigger, &tokens));
            }
        }
        self.hearings
            .heard_triggers
            .retain(|t| t.end + TRIGGERS_KEPT_SECS > position);
        at_once
    }

    /// Asks the checks `trigger`, decoded as `tokens`, calls for: a short "stop" or "cancel"
    /// asks only whether it was said doubled, as its own phrase is spotted beside it; another
    /// trigger asks the doubled command it completes, reported at once when commands go out
    /// without confirmation.
    fn heard_trigger(&mut self, trigger: Spotted, tokens: &str) -> Option<ServerEvent> {
        self.record_spotted(&trigger, tokens);
        log::info!(
            "trigger \"{}\" at {:.2}–{:.2}s",
            trigger.phrase,
            trigger.start,
            trigger.end
        );
        let said_end = samples_in(trigger.end);
        if !keywords::served_by(&trigger.phrase)
            .is_some_and(|phrase| CHECKED_WHEN_SPOTTED.contains(&phrase))
        {
            self.jobs.spotted_checks.spotted_short(said_end);
            return None;
        }
        let at_once = self.doubled_trigger(&trigger);
        self.jobs.spotted_checks.spotted(said_end);
        self.hear_misheard_again(&trigger);
        self.hear_pending_again(&trigger);
        self.hearings.heard_triggers.push(trigger);
        at_once
    }

    /// Asks the check of the doubled command `trigger` completes at once; reported at once too
    /// when commands go out without confirmation.
    fn doubled_trigger(&mut self, trigger: &Spotted) -> Option<ServerEvent> {
        let phrase = keywords::served_by(&trigger.phrase)?;
        let served = Spotted { phrase: phrase.to_owned(), ..trigger.clone() };
        self.ask_checks_of(&served)
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
