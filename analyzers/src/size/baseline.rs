//! The size baseline: every unit over budget, by kind, with its measure (0 for a kind only clippy measures).

use super::budget::{Kind, Measures};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub type Baseline = BTreeMap<(Kind, String), usize>;

const HEADER: &str = "\
# Every unit of Rust code over a size or complexity budget (AGENTS.md § Architecture rules), with its measure.
# A listed unit may shrink, never grow; once within budget it leaves this file. `just size-baseline` rewrites it
# from the code: a rewrite that adds or raises an entry is reviewed like code. Functions listed under a clippy
# lint carry `#[expect(clippy::<lint>, reason = \"size baseline\")]`; excessive_nesting and cognitive_complexity
# have no measure here (0), clippy alone measures them.
";

/// Reads a baseline from its TOML text, or the reason it is malformed.
pub fn parse(text: &str) -> Result<Baseline, String> {
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("size baseline: not TOML: {e}"))?;
    let mut baseline = Baseline::new();
    for kind in Kind::ALL {
        let Some(units) = table.get(kind.key()) else {
            continue;
        };
        let units = units
            .as_table()
            .ok_or_else(|| format!("size baseline: [{}] is not a table", kind.key()))?;
        for (id, measure) in units {
            let measure = measure
                .as_integer()
                .and_then(|m| usize::try_from(m).ok())
                .ok_or_else(|| format!("size baseline: {id} has no measure of zero or more"))?;
            baseline.insert((kind, id.clone()), measure);
        }
    }
    Ok(baseline)
}

/// The baseline the code holds now: every measured unit over budget and every size-baseline attribute.
pub fn from_measures(measures: &Measures) -> Baseline {
    let over = measures
        .units
        .iter()
        .filter(|((kind, _), measure)| over_budget(*kind, **measure));
    let mut baseline: Baseline = over
        .map(|(unit, measure)| (unit.clone(), *measure))
        .collect();
    for (kind, id) in &measures.markers {
        baseline.entry((*kind, id.clone())).or_insert(0);
    }
    baseline
}

pub fn over_budget(kind: Kind, measure: usize) -> bool {
    kind.budget().is_some_and(|budget| measure > budget)
}

/// The TOML text of a baseline, one table per kind, units sorted.
pub fn render(baseline: &Baseline) -> String {
    let mut text = HEADER.to_owned();
    for kind in Kind::ALL {
        let _ = write!(text, "\n[{}]\n", kind.key());
        for ((_, id), measure) in baseline.iter().filter(|((k, _), _)| *k == kind) {
            let _ = writeln!(text, "{} = {measure}", toml::Value::String(id.clone()));
        }
    }
    text
}
