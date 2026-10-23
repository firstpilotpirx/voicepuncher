//! The confirmations the command spotting service cut short, read from its log: a start cue
//! that never came names the copies that could not confirm the command in time.

#[cfg(test)]
mod tests;
mod watch;

pub use watch::{cut_short, report, Watch};
