//! Waiting in a scenario until its run has logged an event.

use std::fmt::Debug;
use std::time::{Duration, Instant};

/// Waits up to `within` until the events `events` returns, from index `from` on, hold `wanted`.
pub fn wait_for_event<E: PartialEq + Debug>(
    events: impl Fn() -> Vec<E>,
    from: usize,
    wanted: &E,
    within: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + within;
    while !events()[from..].contains(wanted) {
        if Instant::now() >= deadline {
            anyhow::bail!("{wanted:?} not seen; events: {:?}", events());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
