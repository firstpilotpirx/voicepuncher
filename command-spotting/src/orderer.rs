//! Puts spotted phrases and speech segments in time order: a speech segment is known only
//! once it ended, a phrase may be spotted before or shortly after the end of its segment.

use crate::protocol::ServerEvent;

/// How far past the end of its speech segment a phrase may end, in seconds: the spotter
/// places a phrase end up to half a second past where the voice activity detector ends
/// quiet speech.
const TAIL_SLACK: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct Spotted {
    pub phrase: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
}

impl Segment {
    /// A phrase belongs to the segment it ends in. Its start is not used: in noise the
    /// spotter may fire the first token early and stretch the phrase back before the speech.
    fn holds(self, spotted: &Spotted) -> bool {
        spotted.end >= self.start && spotted.end <= self.end + TAIL_SLACK
    }
}

#[derive(Debug, Default)]
pub struct EventOrderer {
    pending: Vec<Spotted>,
    last_segment: Option<Segment>,
}

impl EventOrderer {
    /// A phrase spotted in the segment that just ended goes out at once; any other waits
    /// for its segment to end.
    pub fn phrase(&mut self, spotted: Spotted) -> Vec<ServerEvent> {
        if self.last_segment.is_some_and(|s| s.holds(&spotted)) {
            return vec![phrase_event(spotted)];
        }
        self.pending.push(spotted);
        Vec::new()
    }

    /// The segment's start, the phrases spotted in it, then its end.
    pub fn segment(&mut self, segment: Segment) -> Vec<ServerEvent> {
        let mut events = vec![ServerEvent::SpeechStart { t: segment.start }];
        let pending = std::mem::take(&mut self.pending);
        let (inside, outside): (Vec<Spotted>, Vec<Spotted>) =
            pending.into_iter().partition(|p| segment.holds(p));
        events.extend(inside.into_iter().map(phrase_event));
        self.pending = outside
            .into_iter()
            .filter(|p| p.start > segment.end)
            .collect();
        events.push(ServerEvent::SpeechEnd { t: segment.end });
        self.last_segment = Some(segment);
        events
    }
}

fn phrase_event(spotted: Spotted) -> ServerEvent {
    ServerEvent::Phrase {
        phrase: spotted.phrase,
        start: spotted.start,
        end: spotted.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spotted(start: f32, end: f32) -> Spotted {
        Spotted {
            phrase: "press enter".into(),
            start,
            end,
        }
    }

    fn phrase(start: f32, end: f32) -> ServerEvent {
        ServerEvent::Phrase {
            phrase: "press enter".into(),
            start,
            end,
        }
    }

    #[test]
    fn should_report_a_phrase_spotted_during_its_segment_between_start_and_end() {
        // Given
        let mut orderer = EventOrderer::default();
        // When
        let first = orderer.phrase(spotted(1.1, 1.8));
        let then = orderer.segment(Segment {
            start: 1.0,
            end: 1.9,
        });
        // Then
        assert!(first.is_empty());
        assert_eq!(
            then,
            [
                ServerEvent::SpeechStart { t: 1.0 },
                phrase(1.1, 1.8),
                ServerEvent::SpeechEnd { t: 1.9 }
            ]
        );
    }

    #[test]
    fn should_report_a_phrase_spotted_after_its_segment_ended_at_once() {
        // Given
        let mut orderer = EventOrderer::default();
        orderer.segment(Segment {
            start: 1.0,
            end: 1.9,
        });
        // When
        let events = orderer.phrase(spotted(1.1, 1.8));
        // Then
        assert_eq!(events, [phrase(1.1, 1.8)]);
    }

    #[test]
    fn should_keep_a_phrase_of_a_later_segment_waiting() {
        // Given
        let mut orderer = EventOrderer::default();
        orderer.phrase(spotted(5.1, 5.8));
        // When
        let first = orderer.segment(Segment {
            start: 1.0,
            end: 1.9,
        });
        let later = orderer.segment(Segment {
            start: 5.0,
            end: 5.9,
        });
        // Then
        assert!(!first.contains(&phrase(5.1, 5.8)));
        assert!(later.contains(&phrase(5.1, 5.8)));
    }

    #[test]
    fn should_drop_a_phrase_from_before_a_segment() {
        // Given
        let mut orderer = EventOrderer::default();
        orderer.phrase(spotted(0.1, 0.5));
        // When
        orderer.segment(Segment {
            start: 3.0,
            end: 3.9,
        });
        let later = orderer.segment(Segment {
            start: 5.0,
            end: 5.9,
        });
        // Then
        assert!(!later.contains(&phrase(0.1, 0.5)));
    }

    #[test]
    fn should_keep_a_phrase_whose_start_was_stretched_before_its_segment() {
        // Given the spotter placed the phrase start half a second before the speech began
        let mut orderer = EventOrderer::default();
        orderer.phrase(spotted(25.5, 26.75));
        // When
        let events = orderer.segment(Segment {
            start: 26.0,
            end: 27.05,
        });
        // Then
        assert!(events.contains(&phrase(25.5, 26.75)), "{events:?}");
    }
}
