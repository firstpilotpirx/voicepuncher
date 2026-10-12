use std::time::{Duration, Instant};

use super::voice_command::{SpokenCommand, VoiceCommand};

/// What the command spotting service reports about the audio it hears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpottingEvent {
    SpeechStarted(Instant),
    SpeechEnded(Instant),
    PhraseSpotted {
        phrase: String,
        start: Instant,
        end: Instant,
    },
}

/// Longest speech allowed before the phrase in its segment for the phrase to count as the
/// whole utterance.
const LEAD_MARGIN: Duration = Duration::from_millis(300);
/// Longest speech allowed after the phrase: the voice activity detector holds quiet speech
/// a little longer than the spotter places its end.
const TAIL_MARGIN: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy)]
struct Spotted {
    command: Option<VoiceCommand>,
    start: Instant,
    end: Instant,
}

#[derive(Debug, Clone, Copy)]
struct ClosedSegment {
    start: Instant,
    end: Instant,
    spotted_count: usize,
}

/// Turns spotting events into voice commands, keeping only a phrase spoken on its own:
/// the phrase fills its speech segment, with a pause before and after it.
#[derive(Debug, Default)]
pub struct VoiceCommandRecognizer {
    open_segment_start: Option<Instant>,
    spotted_in_open_segment: Vec<Spotted>,
    last_closed: Option<ClosedSegment>,
}

impl VoiceCommandRecognizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, event: SpottingEvent) -> Option<SpokenCommand> {
        match event {
            SpottingEvent::SpeechStarted(at) => {
                self.open_segment_start = Some(at);
                self.spotted_in_open_segment.clear();
                self.last_closed = None;
                None
            }
            SpottingEvent::SpeechEnded(at) => self.close_segment(at),
            SpottingEvent::PhraseSpotted { phrase, start, end } => {
                let spotted = Spotted {
                    command: VoiceCommand::from_phrase(&phrase),
                    start,
                    end,
                };
                self.spot(spotted)
            }
        }
    }

    fn close_segment(&mut self, end: Instant) -> Option<SpokenCommand> {
        let start = self.open_segment_start.take()?;
        let spotted = std::mem::take(&mut self.spotted_in_open_segment);
        let segment = ClosedSegment {
            start,
            end,
            spotted_count: spotted.len(),
        };
        self.last_closed = Some(segment);
        match spotted.as_slice() {
            [only] => fills(segment, *only),
            _ => None,
        }
    }

    /// A phrase may be reported before or shortly after the end of its segment; one that
    /// began before the segment ended belongs to it, even if the spotter placed its end later.
    fn spot(&mut self, spotted: Spotted) -> Option<SpokenCommand> {
        if self.open_segment_start.is_some() {
            self.spotted_in_open_segment.push(spotted);
            return None;
        }
        let segment = self.last_closed.as_mut()?;
        if spotted.start > segment.end {
            return None;
        }
        segment.spotted_count += 1;
        let segment = *segment;
        (segment.spotted_count == 1)
            .then(|| fills(segment, spotted))
            .flatten()
    }
}

fn fills(segment: ClosedSegment, spotted: Spotted) -> Option<SpokenCommand> {
    let lead = spotted.start.saturating_duration_since(segment.start);
    let tail = segment.end.saturating_duration_since(spotted.end);
    if lead > LEAD_MARGIN || tail > TAIL_MARGIN {
        return None;
    }
    spotted.command.map(|command| SpokenCommand {
        command,
        started_at: spotted.start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, secs: f32) -> Instant {
        base + Duration::from_secs_f32(secs)
    }

    fn phrase(base: Instant, text: &str, start: f32, end: f32) -> SpottingEvent {
        SpottingEvent::PhraseSpotted {
            phrase: text.into(),
            start: at(base, start),
            end: at(base, end),
        }
    }

    fn feed_all(events: Vec<SpottingEvent>) -> Vec<SpokenCommand> {
        let mut recognizer = VoiceCommandRecognizer::new();
        events
            .into_iter()
            .filter_map(|e| recognizer.feed(e))
            .collect()
    }

    #[test]
    fn should_recognize_a_phrase_that_fills_its_speech_segment() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "press enter", 1.1, 1.8),
            SpottingEvent::SpeechEnded(at(base, 1.9)),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert_eq!(
            commands,
            vec![SpokenCommand {
                command: VoiceCommand::PressEnter,
                started_at: at(base, 1.1)
            }]
        );
    }

    #[test]
    fn should_recognize_a_phrase_reported_after_its_segment_ended() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            SpottingEvent::SpeechEnded(at(base, 1.9)),
            phrase(base, "stop dictation", 1.1, 1.8),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, VoiceCommand::StopDictation);
    }

    #[test]
    fn should_recognize_a_phrase_reported_after_its_segment_that_sticks_out_of_it() {
        // Given the spotter placed the phrase end 0.2 s after the speech ended
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            SpottingEvent::SpeechEnded(at(base, 1.9)),
            phrase(base, "press enter", 0.9, 2.1),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, VoiceCommand::PressEnter);
    }

    #[test]
    fn should_not_take_a_phrase_that_began_after_the_last_segment_ended() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            SpottingEvent::SpeechEnded(at(base, 1.9)),
            phrase(base, "press enter", 2.5, 3.2),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert!(commands.is_empty());
    }

    #[test]
    fn should_ignore_a_phrase_with_speech_before_it() {
        // Given "I will press enter"
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "press enter", 1.8, 2.5),
            SpottingEvent::SpeechEnded(at(base, 2.6)),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert!(commands.is_empty());
    }

    #[test]
    fn should_ignore_a_phrase_with_speech_after_it() {
        // Given "press enter when I am done"
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "press enter", 1.1, 1.8),
            SpottingEvent::SpeechEnded(at(base, 3.0)),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert!(commands.is_empty());
    }

    #[test]
    fn should_ignore_two_phrases_in_one_segment() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "press enter", 1.1, 1.5),
            phrase(base, "clear all text", 1.5, 1.8),
            SpottingEvent::SpeechEnded(at(base, 1.9)),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert!(commands.is_empty());
    }

    #[test]
    fn should_not_recognize_a_phrase_before_its_segment_ends() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "press enter", 1.1, 1.8),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert!(commands.is_empty());
    }

    #[test]
    fn should_ignore_an_unknown_phrase() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "open the door", 1.1, 1.8),
            SpottingEvent::SpeechEnded(at(base, 1.9)),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert!(commands.is_empty());
    }

    #[test]
    fn should_recognize_the_next_phrase_after_an_ignored_segment() {
        // Given
        let base = Instant::now();
        let events = vec![
            SpottingEvent::SpeechStarted(at(base, 1.0)),
            phrase(base, "press enter", 1.8, 2.5),
            SpottingEvent::SpeechEnded(at(base, 2.6)),
            SpottingEvent::SpeechStarted(at(base, 4.0)),
            phrase(base, "clear all text", 4.1, 4.9),
            SpottingEvent::SpeechEnded(at(base, 5.0)),
        ];
        // When
        let commands = feed_all(events);
        // Then
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, VoiceCommand::ClearAllText);
    }
}
