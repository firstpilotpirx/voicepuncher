//! `dev-tools seams-stage [base]`: the chunk seams stage `full-check` runs and why — `e2e-seams — <reason>` (the
//! whole seam matrix) or `e2e-seams-sample — <reason>` (its sample), by `accuracy-stage`'s rules over the
//! `chunking_paths` of `.config/stages.toml`.

use crate::accuracy::{choose_stage, print_choice, Sampled};
use crate::Outcome;

const SEAMS: Sampled = Sampled {
    whole: "e2e-seams",
    sample: "e2e-seams-sample",
    table: "chunking_paths",
    path: "chunking path",
};

pub fn command(args: &[String]) -> Outcome {
    print_choice(args, SEAMS.table, |root, base, patterns| {
        Ok(choose_stage(root, base, &SEAMS, patterns))
    })
}
