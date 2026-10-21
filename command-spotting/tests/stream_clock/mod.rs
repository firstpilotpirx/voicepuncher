//! Time measured in the speech fed, not on the wall clock: a confirmation comes back a set
//! time after it is asked in the speech, however late or in what bursts the machine feeds it.
//! The feeder moves the clock and waits, before each frame, until every confirmation is either
//! answered and taken or held for a later sample, so the speech never runs ahead of an answer
//! that is due.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use voicepuncher_command_spotting::confirm::{Hear, Heard};
use voicepuncher_command_spotting::engine::{Session, SAMPLE_RATE};
use voicepuncher_command_spotting::protocol::ServerEvent;

/// How often the feeder looks again while confirmations are still on their way.
const SETTLING_STEP: Duration = Duration::from_millis(1);

#[derive(Default)]
struct Moment {
    /// The sample at which a confirmation asked now counts as asked.
    asking_at: usize,
    /// Confirmations whose answer is due at or before this sample come back.
    released_to: usize,
    /// Every confirmation comes back: the speech has ended.
    all_released: bool,
    /// The worker threads holding an answer for a later sample, by thread name.
    holding: HashMap<String, usize>,
}

#[derive(Default)]
pub struct StreamClock {
    moment: Mutex<Moment>,
    released: Condvar,
}

impl StreamClock {
    /// The sample a confirmation asked now counts as asked at.
    pub fn now(&self) -> usize {
        self.moment.lock().unwrap().asking_at
    }

    /// Holds the calling worker thread until the speech has reached `sample`.
    pub fn wait_until(&self, sample: usize) {
        let thread = std::thread::current().name().unwrap_or("").to_owned();
        let mut moment = self.moment.lock().unwrap();
        if moment.all_released || sample <= moment.released_to {
            return;
        }
        moment.holding.insert(thread, sample);
        let _released = self
            .released
            .wait_while(moment, |m| !m.all_released && sample > m.released_to)
            .unwrap();
    }

    /// Feeds one frame of `frame` at `fed` samples, then releases the answers due by its end
    /// and takes every one of them: the events the frame brought.
    pub fn feed(&self, session: &mut Session<'_>, frame: &[f32], fed: usize) -> Vec<ServerEvent> {
        let end = fed + frame.len();
        self.moment.lock().unwrap().asking_at = end;
        let mut events = session.accept(frame);
        self.release(|m| m.released_to = end);
        events.extend(self.settled(session));
        events
    }

    /// The speech has ended: every held answer comes back.
    pub fn release_all(&self) {
        self.release(|m| m.all_released = true);
    }

    fn release(&self, change: impl FnOnce(&mut Moment)) {
        let mut moment = self.moment.lock().unwrap();
        change(&mut moment);
        let (to, all) = (moment.released_to, moment.all_released);
        moment.holding.retain(|_, due| !all && *due > to);
        self.released.notify_all();
    }

    /// The events of every answer that came back, once each confirmation is answered and
    /// taken or held for a later sample.
    fn settled(&self, session: &mut Session<'_>) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        loop {
            events.extend(session.poll());
            let load = session.confirmation_load();
            let unanswered: usize = load.unanswered.iter().map(|(_, n)| n).sum();
            let busy = {
                let moment = self.moment.lock().unwrap();
                load.unanswered
                    .iter()
                    .any(|(thread, n)| *n > 0 && !moment.holding.contains_key(thread))
            };
            // Every answer given is taken, and every thread with a job holds its answer for later
            if load.untaken == unanswered && !busy {
                return events;
            }
            std::thread::sleep(SETTLING_STEP);
        }
    }
}

/// A hearing whose answers come back `delay` after they are asked, in the speech: `inner`
/// hears the clip at once, with `deadline` in place of the engine's when given.
pub struct Clocked {
    pub inner: std::sync::Arc<dyn Hear>,
    pub clock: std::sync::Arc<StreamClock>,
    pub delay: Duration,
    pub deadline: Option<Duration>,
}

impl Hear for Clocked {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        let asked = self.clock.now();
        let heard = self.inner.hear(clip, self.deadline.unwrap_or(deadline));
        self.clock.wait_until(asked + samples(self.delay));
        heard
    }
}

/// `duration` of speech, in samples.
pub fn samples(duration: Duration) -> usize {
    (duration.as_secs_f64() * f64::from(SAMPLE_RATE)) as usize
}
