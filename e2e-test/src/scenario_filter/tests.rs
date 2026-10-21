//! Runs cucumber in process on a fixture feature with the options and the filter `split` returns,
//! as the harnesses do, and checks which scenarios ran.

use std::env::VarError;
use std::sync::{Arc, Mutex};

use cucumber::cli::{Empty, Opts, Parser as _};
use cucumber::writer::{Basic, Coloring, Verbosity};
use cucumber::World as _;

use super::*;

const FIXTURE: &str = r#"
@gates_fixture
Feature: Gates fixture

  Scenario: ungated fixture
    Given a step

  @wip
  Scenario: wip fixture
    Given a step

  Scenario Outline: voice samples fixture
    Given a step

    @voice_samples
    Examples:
      | row |
      | one |

  @real_keyboard
  Rule: real keyboard fixture rule

    Scenario: real keyboard fixture
      Given a step
"#;
const UNGATED: &str = "ungated fixture";
const VOICE_SAMPLES: &str = "voice samples fixture";
const REAL_KEYBOARD: &str = "real keyboard fixture";

#[derive(Debug, Default, cucumber::World)]
struct FixtureWorld;

#[cucumber::given("a step")]
fn a_step(_world: &mut FixtureWorld) {}

fn closed_gates() -> Vec<Gate> {
    vec![
        Gate::wip(),
        Gate::voice_samples(Err(VarError::NotPresent)),
        Gate::real_keyboard(Err(VarError::NotPresent)),
    ]
}

fn open_gates() -> Vec<Gate> {
    vec![
        Gate::wip(),
        Gate::voice_samples(Ok("/recordings".to_string())),
        Gate::real_keyboard(Ok("1".to_string())),
    ]
}

/// The names of the fixture scenarios that ran, sorted, under this cucumber command line.
fn scenarios_run(command_line: &[&str], gates: Vec<Gate>) -> Vec<String> {
    scenarios_rerun(command_line, gates, None)
}

/// The names of the fixture scenarios that ran, sorted, under this command line and the rerun
/// names `rerun` (the value of the rerun variable, `None` when unset).
fn scenarios_rerun(command_line: &[&str], gates: Vec<Gate>, rerun: Option<&str>) -> Vec<String> {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("gates.feature"), FIXTURE).expect("fixture feature");
    let program = std::iter::once("harness");
    let opts = Opts::<_, _, _, Empty>::parse_from(program.chain(command_line.iter().copied()));
    let (opts, filter) = split(opts, gates);
    let filter = filter.rerun_only(rerun_names(
        rerun.map(str::to_string).ok_or(VarError::NotPresent),
    ));
    let names = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&names);
    futures::executor::block_on(
        FixtureWorld::cucumber()
            .with_writer(Basic::new(
                std::io::sink(),
                Coloring::Never,
                Verbosity::Default,
            ))
            .after(move |_feature, _rule, scenario, _finished, _world| {
                recorded.lock().expect("names").push(scenario.name.clone());
                Box::pin(async {})
            })
            .with_cli(opts)
            .filter_run(dir.path(), move |f, r, s| filter.runs(f, r, s)),
    );
    let mut ran = names.lock().expect("names").clone();
    ran.sort();
    ran
}

/// The fixture scenarios that run under this command line are exactly `expected`; the error
/// names the scenarios that ran. A `Result`, not a panic: cucumber silences the panic hook while
/// it runs, which would hide a failed assertion of a test beside it.
fn expect_runs(command_line: &[&str], gates: Vec<Gate>, expected: &[&str]) -> Result<(), String> {
    let ran = scenarios_run(command_line, gates);
    if ran == expected {
        Ok(())
    } else {
        Err(format!(
            "under {command_line:?} ran {ran:?}, expected {expected:?}"
        ))
    }
}

#[test]
fn should_run_only_the_ungated_scenario_when_tags_select_every_scenario_and_the_gates_are_closed(
) -> Result<(), String> {
    expect_runs(&["--tags", "@gates_fixture"], closed_gates(), &[UNGATED])
}

#[test]
fn should_run_only_the_ungated_scenario_when_a_name_selects_every_scenario_and_the_gates_are_closed(
) -> Result<(), String> {
    expect_runs(&["--name", "fixture"], closed_gates(), &[UNGATED])
}

#[test]
fn should_run_no_wip_scenario_when_tags_select_it() -> Result<(), String> {
    expect_runs(&["--tags", "@wip"], open_gates(), &[])
}

#[test]
fn should_run_no_wip_scenario_when_a_name_selects_it() -> Result<(), String> {
    expect_runs(&["--name", "wip"], open_gates(), &[])
}

#[test]
fn should_run_no_wip_scenario_without_a_selection() -> Result<(), String> {
    expect_runs(&[], open_gates(), &[REAL_KEYBOARD, UNGATED, VOICE_SAMPLES])
}

#[test]
fn should_run_a_gated_scenario_when_its_gate_is_open_and_the_selection_matches_it(
) -> Result<(), String> {
    expect_runs(
        &["--tags", "@voice_samples"],
        open_gates(),
        &[VOICE_SAMPLES],
    )?;
    expect_runs(
        &["--tags", "@real_keyboard"],
        open_gates(),
        &[REAL_KEYBOARD],
    )
}

#[test]
fn should_run_what_the_closed_gates_admit_without_a_selection() -> Result<(), String> {
    expect_runs(&[], closed_gates(), &[UNGATED])
}

#[test]
fn should_leave_out_the_ungated_scenario_when_the_selection_misses_it() -> Result<(), String> {
    expect_runs(&["--name", "keyboard"], open_gates(), &[REAL_KEYBOARD])
}

/// Like `expect_runs`, with the rerun variable set to `rerun`.
fn expect_reruns(
    command_line: &[&str],
    gates: Vec<Gate>,
    rerun: &str,
    expected: &[&str],
) -> Result<(), String> {
    let ran = scenarios_rerun(command_line, gates, Some(rerun));
    if ran == expected {
        Ok(())
    } else {
        Err(format!(
            "under {command_line:?} with rerun {rerun:?} ran {ran:?}, expected {expected:?}"
        ))
    }
}

#[test]
fn should_run_exactly_the_rerun_names_instead_of_the_tags() -> Result<(), String> {
    expect_reruns(
        &["--tags", "@real_keyboard"],
        open_gates(),
        &format!("{UNGATED}\n{VOICE_SAMPLES}\n"),
        &[UNGATED, VOICE_SAMPLES],
    )
}

#[test]
fn should_match_a_rerun_name_whole_not_as_a_part() -> Result<(), String> {
    expect_reruns(&[], open_gates(), "fixture", &[])
}

#[test]
fn should_hold_back_a_rerun_name_its_closed_gate_holds() -> Result<(), String> {
    expect_reruns(
        &[],
        closed_gates(),
        &format!("{REAL_KEYBOARD}\n{UNGATED}"),
        &[UNGATED],
    )
}

#[test]
fn should_keep_the_command_line_selection_when_the_rerun_variable_is_empty() -> Result<(), String> {
    expect_reruns(
        &["--tags", "@real_keyboard"],
        open_gates(),
        "\n",
        &[REAL_KEYBOARD],
    )
}

#[test]
fn should_close_the_voice_samples_gate_when_the_variable_is_unset() {
    let gate = Gate::voice_samples(Err(VarError::NotPresent));
    assert!(!gate.is_open());
    assert_eq!(
        gate.line(),
        Some("GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping @voice_samples scenarios")
    );
}

#[test]
fn should_close_the_voice_samples_gate_when_the_variable_is_empty() {
    let gate = Gate::voice_samples(Ok(String::new()));
    assert!(!gate.is_open());
    assert_eq!(
        gate.line(),
        Some("GATED: VOICEPUNCHER_VOICE_SAMPLES empty, skipping @voice_samples scenarios")
    );
}

#[test]
fn should_open_the_voice_samples_gate_when_the_variable_names_a_path() {
    let gate = Gate::voice_samples(Ok("/recordings".to_string()));
    assert!(gate.is_open());
    assert_eq!(gate.line(), None);
}

#[test]
fn should_close_the_real_keyboard_gate_only_when_the_variable_is_unset() {
    let closed = Gate::real_keyboard(Err(VarError::NotPresent));
    assert!(!closed.is_open());
    assert_eq!(
        closed.line(),
        Some("GATED: VOICEPUNCHER_REAL_KEYBOARD_IT not set, skipping @real_keyboard scenarios")
    );
    assert!(Gate::real_keyboard(Ok(String::new())).is_open());
}

#[test]
fn should_close_the_signing_identity_gate_when_the_variable_is_unset() {
    let gate = Gate::signing_identity(Err(VarError::NotPresent));
    assert!(!gate.is_open());
    assert_eq!(
        gate.line(),
        Some("GATED: VOICEPUNCHER_SIGNING_IDENTITY not set, skipping @signing_identity scenarios")
    );
}

#[test]
fn should_close_the_signing_identity_gate_when_the_variable_is_empty() {
    let gate = Gate::signing_identity(Ok(String::new()));
    assert!(!gate.is_open());
    assert_eq!(
        gate.line(),
        Some("GATED: VOICEPUNCHER_SIGNING_IDENTITY empty, skipping @signing_identity scenarios")
    );
}

#[test]
fn should_open_the_signing_identity_gate_when_the_variable_names_an_identity() {
    let gate = Gate::signing_identity(Ok("voicepuncher".to_string()));
    assert!(gate.is_open());
    assert_eq!(gate.line(), None);
}
