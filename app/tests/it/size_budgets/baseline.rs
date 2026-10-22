//! The size baseline: every unit over budget, by kind, with its measure (0 for a kind only clippy measures).

use super::budget::{Kind, Measures};
use std::collections::BTreeMap;

pub type Baseline = BTreeMap<(Kind, String), usize>;

const HEADER: &str = "\
# Every unit of Rust code over a size or complexity budget (AGENTS.md § Architecture rules), with its measure.
# A listed unit may shrink, never grow; once within budget it leaves this file. `just size-baseline` rewrites it
# from the code: a rewrite that adds or raises an entry is reviewed like code. Functions listed under a clippy
# lint carry `#[expect(clippy::<lint>, reason = \"size baseline\")]`; excessive_nesting and cognitive_complexity
# have no measure here (0), clippy alone measures them.
";

/// Reads a baseline from its TOML text; panics with the reason when the text is malformed.
pub fn parse(text: &str) -> Baseline {
    let table: toml::Table = text.parse().expect("size baseline: not TOML");
    let mut baseline = Baseline::new();
    for kind in Kind::ALL {
        let Some(units) = table.get(kind.key()) else {
            continue;
        };
        let units = units
            .as_table()
            .expect("size baseline: a kind is not a table");
        for (id, measure) in units {
            let measure = measure
                .as_integer()
                .expect("size baseline: a measure is not an integer");
            baseline.insert(
                (kind, id.clone()),
                usize::try_from(measure).expect("negative measure"),
            );
        }
    }
    baseline
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
        text.push_str(&format!("\n[{}]\n", kind.key()));
        for ((_, id), measure) in baseline.iter().filter(|((k, _), _)| *k == kind) {
            text.push_str(&format!(
                "{} = {measure}\n",
                toml::Value::String(id.clone())
            ));
        }
    }
    text
}
