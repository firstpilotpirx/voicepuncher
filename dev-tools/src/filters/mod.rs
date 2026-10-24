//! The nextest filtersets of the test selections: `test` (every target but the E2E harness), a smoke part (its
//! `it` modules and the E2E harness) and the `it` modules of a stage, each narrowed to a rerun's selection
//! (`VOICEPUNCHER_RERUN_FILTER`), without the frozen tests (`VOICEPUNCHER_FROZEN_FILTER`) and without the tests
//! of the closed needs (`gates`).

pub mod gates;
#[cfg(test)]
mod tests;

use crate::context::{var, Context};
use crate::Outcome;

/// The targets a selection's needs are counted over, when the caller names none.
const SMOKE_TARGETS: &str =
    "-p voicepuncher -p voicepuncher-command-spotting -p voicepuncher-e2e-test --test it --test cucumber_e2e";
const SEAMS_TARGETS: &str = "-p voicepuncher --test it";

/// A selection before the rerun, frozen and needs narrowing.
pub enum Selection<'a> {
    Test,
    Smoke(&'a [String]),
    It(&'a [String]),
}

impl Selection<'_> {
    fn base(&self) -> String {
        match self {
            Self::Test => "not binary_id(voicepuncher-e2e-test::cucumber_e2e)".into(),
            Self::Smoke(modules) => {
                let parts: Vec<String> = modules
                    .iter()
                    .map(|module| format!("(binary(=it) & test(/^{module}::/)) | "))
                    .collect();
                format!("{}binary(=cucumber_e2e)", parts.concat())
            }
            Self::It(modules) => {
                let parts: Vec<String> = modules
                    .iter()
                    .map(|module| format!("test(/^{module}::/)"))
                    .collect();
                format!("binary(=it) & ({})", parts.join(" | "))
            }
        }
    }

    /// The filterset narrowed to a rerun's selection and without the frozen tests; `test` appends its clauses,
    /// the others wrap (a second `-E` would widen a selection: nextest joins several with "or").
    pub fn narrowed(&self, rerun: Option<&str>, frozen: Option<&str>) -> String {
        let mut filter = self.base();
        for (clause, negated) in [(rerun, ""), (frozen, "not ")] {
            let Some(clause) = clause else { continue };
            filter = match self {
                Self::Test => format!("{filter} & {negated}({clause})"),
                _ => format!("({filter}) & {negated}({clause})"),
            };
        }
        filter
    }

    fn default_targets(&self) -> &'static str {
        match self {
            Self::Test => "--workspace",
            Self::Smoke(_) => SMOKE_TARGETS,
            Self::It(_) => SEAMS_TARGETS,
        }
    }
}

/// `dev-tools filter test|smoke|it [module…]`: prints the selection's filterset.
pub fn command(args: &[String]) -> Outcome {
    let (kind, modules) = args.split_first().ok_or("filter: name test, smoke or it")?;
    let modules: Vec<String> = modules
        .iter()
        .flat_map(|list| list.split_whitespace().map(String::from))
        .collect();
    let selection = match kind.as_str() {
        "test" => Selection::Test,
        "smoke" => Selection::Smoke(&modules),
        "it" => Selection::It(&modules),
        other => {
            return Err(format!(
                "filter: unknown selection {other}: test, smoke or it"
            ))
        }
    };
    let filter = selection.narrowed(
        var("VOICEPUNCHER_RERUN_FILTER").as_deref(),
        var("VOICEPUNCHER_FROZEN_FILTER").as_deref(),
    );
    let targets =
        var("VOICEPUNCHER_GATES_TARGETS").unwrap_or_else(|| selection.default_targets().into());
    println!("{}", gates::gated(&Context::from_env(), &targets, &filter)?);
    Ok(0)
}

/// `dev-tools gated-filter <targets> <filterset>`: prints the filterset without the tests of the closed needs.
pub fn gated_command(args: &[String]) -> Outcome {
    let [targets, filter] = args else {
        return Err("gated-filter: give the cargo targets and the filterset".into());
    };
    println!("{}", gates::gated(&Context::from_env(), targets, filter)?);
    Ok(0)
}
