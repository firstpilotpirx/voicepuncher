use super::*;
use crate::matcher::tests::{dictating, standby};

mod chains;
mod placements;

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// Each command with the indices of its first and last word.
fn found(commands: &[(&str, usize, usize)], placement: Placement) -> Found {
    Found {
        commands: commands.iter().map(|(c, _, _)| (*c).to_owned()).collect(),
        spans: commands.iter().map(|&(_, f, l)| (f, l)).collect(),
        first: commands[0].1,
        last: commands[commands.len() - 1].2,
        placement,
    }
}
