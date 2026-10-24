//! The checks the command spotting service asks of a running utterance when it spots
//! "start" or "restart" in it, through its `SpottedChecks`.

use anyhow::{bail, Result};
use cucumber::{given, then, when};

use voicepuncher_command_spotting::engine::spotted::{Round, SpottedChecks};

use super::run;
use crate::E2eWorld;

/// Where the phrase spotted later ends, in samples: 3 s into the stream.
const LATER_SAID_END: usize = 48_000;
/// Where the earlier phrase ends, and how far its first check heard.
const EARLIER_SAID_END: usize = 16_000;
const EARLIER_HEARD_TO: usize = 32_000;
const EARLIER_UTTERANCE: u64 = 1;

/// The checks of a scenario, and the rounds asked once every one fell due.
#[derive(Debug, Default)]
pub struct SpottedRun {
    checks: SpottedChecks,
    asked: Vec<Round>,
}

#[given("a check of a spotted \"start\" waits its turn")]
fn check_waits(world: &mut E2eWorld) {
    run(world).spotted.checks.spotted(LATER_SAID_END);
}

#[when("the first check of an earlier spotted \"start\" finds no command")]
fn first_check_finds_nothing(world: &mut E2eWorld) {
    let checks = &mut run(world).spotted.checks;
    checks.found_nothing(EARLIER_SAID_END, EARLIER_HEARD_TO, EARLIER_UTTERANCE);
}

#[then("both the waiting check and the second check of the earlier phrase are asked")]
fn both_asked(world: &mut E2eWorld) -> Result<()> {
    let spotted = &mut run(world).spotted;
    while let Some(due) = spotted.checks.take_due(usize::MAX) {
        spotted.asked.push(due.round);
    }
    // The earlier phrase's second check falls due first
    if spotted.asked != [Round::Second, Round::First] {
        bail!("asked {:?}", spotted.asked);
    }
    Ok(())
}
