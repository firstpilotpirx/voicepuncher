//! Compares what the code measures with the size baseline and names every unit that breaks a budget.

use crate::baseline::{over_budget, Baseline};
use crate::budget::{Kind, Measures};

/// Every violation, one line each: a unit over budget and not listed, a listed unit that grew, a listed unit
/// back within budget or gone, and a size-baseline attribute and its baseline entry without the other.
pub fn violations(measures: &Measures, baseline: &Baseline) -> Vec<String> {
    let mut found = Vec::new();
    for ((kind, id), measure) in &measures.units {
        if let Some(problem) = judge(*kind, *measure, baseline.get(&(*kind, id.clone())).copied()) {
            found.push(format!("{} {id}: {problem}", kind.key()));
        }
    }
    for (kind, id) in baseline.keys() {
        let measured = kind.budget().is_none() || measures.units.contains_key(&(*kind, id.clone()));
        if !measured {
            found.push(format!(
                "{} {id}: listed in the size baseline but gone; remove its entry",
                kind.key()
            ));
        }
    }
    found.extend(attribute_mismatches(measures, baseline));
    found
}

fn judge(kind: Kind, measure: usize, listed: Option<usize>) -> Option<String> {
    let unit = kind.unit_name();
    let budget = kind.budget()?;
    match listed {
        None if over_budget(kind, measure) => Some(format!("{measure} {unit}, over the budget of {budget}")),
        None => None,
        Some(_) if !over_budget(kind, measure) => Some(format!(
            "{measure} {unit}, back within the budget of {budget}; remove its size-baseline entry and attribute"
        )),
        Some(listed) if measure > listed => {
            Some(format!("grew from {listed} to {measure} {unit} (budget {budget}); shrink it"))
        }
        Some(_) => None,
    }
}

fn attribute_mismatches(measures: &Measures, baseline: &Baseline) -> Vec<String> {
    let listed = baseline.keys().filter(|(kind, _)| kind.has_lint());
    let unmarked = listed
        .filter(|unit| !measures.markers.contains(unit))
        .map(|(kind, id)| {
            format!(
                "{} {id}: listed in the size baseline without its size-baseline attribute",
                kind.key()
            )
        });
    let unlisted = measures
        .markers
        .iter()
        .filter(|unit| !baseline.contains_key(unit))
        .map(|(kind, id)| {
            format!(
                "{} {id}: size-baseline attribute without a baseline entry",
                kind.key()
            )
        });
    unmarked.chain(unlisted).collect()
}
