//! Puts spotted phrases and speech segments in time order: a speech segment is known only
//! once it ended, a phrase may be spotted before or shortly after the end of its segment.

use crate::protocol::ServerEvent;

/// How far past the end of its speech segment a phrase may end, in seconds: the spotter
/// places a phrase end up to half a second past where the voice activity detector ends
/// quiet speech.
const TAIL_SLACK: f32 = 0.5;
/// How long after its end a phrase waits for a speech segment: after that the voice
/// activity detector has missed the utterance, which happens with quiet speech in noise.
const UNHELD_AFTER: f32 = 1.0;

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

    /// The phrases no speech segment took although `now` (seconds of audio) is well past
    /// their end; they are dropped from the waiting ones.
    pub fn unheld(&mut self, now: f32) -> Vec<Spotted> {
        let (unheld, waiting): (Vec<Spotted>, Vec<Spotted>) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| now - p.end > UNHELD_AFTER);
        self.pending = waiting;
        unheld
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
        timing: None,
    }
}

#[cfg(test)]
mod tests;
