//! Frozen scenarios: a scenario the frozen list names does not run, whatever selects it.

use std::env::VarError;

use cucumber::cli::{Empty, Opts, Parser as _};

use super::*;

/// The fixture scenarios that run under this command line, the rerun names `rerun` and the
/// frozen names `frozen` (each the value of its variable, `None` when unset) are exactly `expected`.
fn expect_frozen_runs(
    command_line: &[&str],
    rerun: Option<&str>,
    frozen: Option<&str>,
    expected: &[&str],
) -> Result<(), String> {
    let value = |names: Option<&str>| names.map(str::to_string).ok_or(VarError::NotPresent);
    let ran = scenarios_filtered(command_line, open_gates(), |filter| {
        filter
            .rerun_only(scenario_names(value(rerun)))
            .without_frozen(scenario_names(value(frozen)))
    });
    if ran == expected {
        Ok(())
    } else {
        Err(format!(
            "under {command_line:?}, rerun {rerun:?}, frozen {frozen:?} ran {ran:?}, expected {expected:?}"
        ))
    }
}

#[test]
fn should_skip_a_frozen_scenario_and_run_the_others_of_its_feature() -> Result<(), String> {
    expect_frozen_runs(&[], None, Some(UNGATED), &[REAL_KEYBOARD, VOICE_SAMPLES])
}

#[test]
fn should_skip_a_frozen_scenario_its_tags_select() -> Result<(), String> {
    expect_frozen_runs(
        &["--tags", "@voice_samples"],
        None,
        Some(VOICE_SAMPLES),
        &[],
    )
}

#[test]
fn should_skip_a_frozen_scenario_a_rerun_names() -> Result<(), String> {
    let rerun = format!("{UNGATED}\n{REAL_KEYBOARD}");
    expect_frozen_runs(&[], Some(&rerun), Some(UNGATED), &[REAL_KEYBOARD])
}

#[test]
fn should_match_a_frozen_name_whole_not_as_a_part() -> Result<(), String> {
    expect_frozen_runs(
        &[],
        None,
        Some("fixture"),
        &[REAL_KEYBOARD, UNGATED, VOICE_SAMPLES],
    )
}

#[test]
fn should_run_every_scenario_when_the_frozen_variable_is_unset_or_empty() -> Result<(), String> {
    expect_frozen_runs(&[], None, None, &[REAL_KEYBOARD, UNGATED, VOICE_SAMPLES])?;
    expect_frozen_runs(
        &[],
        None,
        Some("\n"),
        &[REAL_KEYBOARD, UNGATED, VOICE_SAMPLES],
    )
}

#[test]
fn should_freeze_a_scenario_outline_as_a_whole() -> Result<(), String> {
    expect_frozen_runs(&[], None, Some(VOICE_SAMPLES), &[REAL_KEYBOARD, UNGATED])
}

/// The fixture feature and the filter a harness without arguments builds under `gates`.
fn fixture_filter(gates: Vec<Gate>) -> (cucumber::gherkin::Feature, ScenarioFilter) {
    let feature =
        cucumber::gherkin::Feature::parse(FIXTURE, cucumber::gherkin::GherkinEnv::default())
            .expect("fixture parses");
    let opts = Opts::<Empty, Empty, Empty, Empty>::parse_from(["harness"]);
    (feature, split(opts, gates).1)
}

/// The `FROZEN:` line `frozen_skip` gives each fixture scenario, by scenario name, under the
/// frozen names `frozen` and every gate open.
fn frozen_skips(frozen: &str) -> Vec<(String, Option<String>)> {
    let (feature, filter) = fixture_filter(open_gates());
    let filter = filter.without_frozen(scenario_names(Ok(frozen.to_string())));
    let top = feature.scenarios.iter().map(|scenario| (None, scenario));
    let ruled = feature.rules.iter().flat_map(|rule| {
        rule.scenarios
            .iter()
            .map(move |scenario| (Some(rule), scenario))
    });
    top.chain(ruled)
        .map(|(rule, scenario)| {
            let line = filter.frozen_skip(&feature, rule, scenario);
            (scenario.name.clone(), line)
        })
        .collect()
}

#[test]
fn should_name_the_frozen_scenario_it_skips_and_no_other() {
    let skips = frozen_skips(UNGATED);
    let named: Vec<_> = skips.iter().filter_map(|(_, line)| line.clone()).collect();
    assert_eq!(
        named,
        [format!(
            "FROZEN: skipping scenario '{UNGATED}' (.config/frozen-tests.toml)"
        )],
        "lines per scenario: {skips:?}"
    );
}

#[test]
fn should_name_no_frozen_scenario_its_gate_holds_back() {
    let (feature, filter) = fixture_filter(vec![Gate::wip()]);
    let filter = filter.without_frozen(Some(vec!["wip fixture".to_string()]));
    let wip = feature
        .scenarios
        .iter()
        .find(|s| s.name == "wip fixture")
        .expect("wip fixture");
    assert_eq!(filter.frozen_skip(&feature, None, wip), None);
}
