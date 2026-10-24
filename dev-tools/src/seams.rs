//! `dev-tools seams-stage [base]`: the chunk seams stage `full-check` runs and why — `e2e-seams — <reason>` (the
//! whole seam matrix) or `e2e-seams-sample — <reason>` (its sample), by `accuracy-stage`'s rules over the
//! `chunking_paths` of `.config/stages.toml`.

use crate::accuracy::{choose_stage, Sampled};
use crate::context::Context;
use crate::Outcome;

const SEAMS: Sampled = Sampled {
    whole: "e2e-seams",
    sample: "e2e-seams-sample",
    table: "chunking_paths",
    path: "chunking path",
};

pub fn command(args: &[String]) -> Outcome {
    let base = args.first().map_or("origin/main", String::as_str);
    let context = Context::from_env();
    let (stage, reason) = choose_stage(&context.root, base, &SEAMS, &context.table(SEAMS.table)?);
    println!("{stage} — {reason}");
    Ok(0)
}
