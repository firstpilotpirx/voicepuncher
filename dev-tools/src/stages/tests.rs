//! The stage table, the model phase's skipped stages, the desktop tests' guard and the order of the checks'
//! stages (former `_test-stage-skip`, `_test-stage-order`).

use super::{is_model, it_real, model, table, test};
use crate::context::{table as justfile_table, Context};
use crate::journal::report::{Report, Row};
use crate::process::Interrupt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const JUSTFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../justfile"));

/// The value of a `name := "…"` line of the `justfile`.
fn variable(name: &str) -> String {
    let start = format!("{name} := \"");
    let line = JUSTFILE
        .lines()
        .find(|line| line.starts_with(&start))
        .unwrap();
    line[start.len()..line.len() - 1].to_string()
}

fn sorted(text: &str) -> Vec<&str> {
    let mut words: Vec<&str> = text.split_whitespace().collect();
    words.sort_unstable();
    words
}

fn script(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn should_print_a_row_per_stage_and_lock_in_the_order_of_the_report() {
    let report = "stage ✓ fmt 1.2\nstage ✓ test 61.7\nrequested e2e 100.0\nlocked e2e 112.5\nstage ✗ e2e-smoke-voice 94.8\nreturned e2e 210.0\n";
    assert_eq!(
        table::lines(report, false, 300.0),
        [
            "  ✓ fmt                1.2s",
            "  ✓ test              61.7s",
            "  ✗ e2e-smoke-voice    94.8s",
            "  ● e2e lock       wait    12.5s, held    97.5s",
        ]
    );
    assert_eq!(table::total(1110.3 - 990.05), "  total              120.2s");
}

#[test]
fn should_show_a_lock_never_taken_and_one_the_caller_held() {
    let report = "requested e2e 100.0\n";
    assert_eq!(
        table::lines(report, false, 110.0),
        ["  ✗ e2e lock       wait    10.0s, not taken"]
    );
    assert_eq!(
        table::lines("requested machine 1.0\n", true, 2.0),
        ["  ● lock           held by the caller"]
    );
}

#[test]
fn should_lock_every_stage_from_the_first_model_stage_on() {
    for stage in ["e2e-smoke-voice", "e2e-cues", "refine-eval", "it-real"] {
        assert!(is_model(stage), "{stage}");
    }
    for stage in [
        "fmt",
        "lint",
        "test",
        "metrics",
        "build",
        "build-e2e",
        "accuracy",
    ] {
        assert!(!is_model(stage), "{stage}");
    }
}

#[test]
fn should_skip_a_stage_exiting_77_and_go_on_without_a_pass() {
    let dir = tempfile::tempdir().unwrap();
    let just = script(
        dir.path(),
        "just",
        "case \"$3\" in _probe-skipped) exit 77 ;; _probe-passed) exit 0 ;; esac\nexit 1",
    );
    let context = Context {
        just: just.to_string_lossy().into_owned(),
        justfile: dir.path().join("justfile"),
        root: dir.path().into(),
    };
    let report = dir.path().join("report");
    let stages = ["probe-skipped".to_string(), "probe-passed".into()];
    assert_eq!(
        model::run(&context, &stages, Some(&report), &Interrupt::watch()),
        0
    );
    let report = Report::parse(&std::fs::read_to_string(&report).unwrap());
    assert!(
        matches!(
            report.rows.get("probe-skipped"),
            Some(Row::Ran { result: "skipped", .. })
        ),
        "{:?}",
        report.rows
    );
    assert!(
        matches!(
            report.rows.get("probe-passed"),
            Some(Row::Ran { result: "passed", .. })
        ),
        "{:?}",
        report.rows
    );
    assert_eq!(report.passed(), ["probe-passed"]);
}

#[test]
fn should_stop_the_model_stages_at_the_first_failure() {
    let dir = tempfile::tempdir().unwrap();
    let just = script(dir.path(), "just", "exit 3");
    let context = Context {
        just: just.to_string_lossy().into_owned(),
        justfile: dir.path().join("justfile"),
        root: dir.path().into(),
    };
    let report = dir.path().join("report");
    let stages = ["first".to_string(), "second".into()];
    assert_eq!(
        model::run(&context, &stages, Some(&report), &Interrupt::watch()),
        3
    );
    let report = Report::parse(&std::fs::read_to_string(&report).unwrap());
    assert!(
        matches!(
            report.rows.get("first"),
            Some(Row::Ran { result: "failed", .. })
        ),
        "{:?}",
        report.rows
    );
    assert!(!report.rows.contains_key("second"), "{:?}", report.rows);
}

#[test]
fn should_name_a_missing_permission_and_leave_out_the_tests_that_press_keys() {
    assert_eq!(
        it_real::missing_lines("Accessibility not granted to Terminal\n"),
        ["⊘ Accessibility not granted to Terminal: the desktop tests are skipped (grant it in System Settings › Privacy & Security, or run just it-real by hand)"]
    );
    assert!(it_real::FILTER.contains("not test(/::needs_real_keyboard::/)"));
    assert_eq!(
        it_real::filter(Some("test(=a)"), Some("f")),
        format!("{} & (test(=a)) & not (f)", it_real::FILTER)
    );
}

#[test]
fn should_read_a_member_package_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"voicepuncher-x\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(
        test::package_name(&dir.path().join("Cargo.toml")),
        "voicepuncher-x"
    );
}

#[test]
fn should_run_voice_control_first_and_the_cues_last() {
    let (check, full) = (
        variable("check_model_stages"),
        variable("full_check_model_stages"),
    );
    assert!(check.starts_with("e2e-smoke-voice "), "{check}");
    assert!(
        full.starts_with("e2e-smoke-voice e2e-regression e2e-resilience "),
        "{full}"
    );
    assert!(full.ends_with(" e2e-seams it-real e2e-cues"), "{full}");
    assert!(variable("e2e_smoke_stages").starts_with("e2e-smoke-voice"));
    assert!(variable("voice_smoke_modules").starts_with("placed_commands voice_forms "));
}

#[test]
fn should_run_the_same_stages_as_before() {
    let check = "e2e-smoke-speech e2e-smoke-refinement e2e-smoke-voice e2e-smoke-services e2e-quick refine-eval";
    assert_eq!(sorted(&variable("check_model_stages")), sorted(check));
    let full = "e2e-smoke-speech e2e-smoke-refinement e2e-smoke-voice e2e-smoke-services e2e-regression e2e-seams it-real e2e-cues e2e-resilience e2e-bundle accuracy refine-eval";
    assert_eq!(sorted(&variable("full_check_model_stages")), sorted(full));
    let smoke = "e2e-smoke-speech e2e-smoke-refinement e2e-smoke-voice e2e-smoke-services";
    assert_eq!(sorted(&variable("e2e_smoke_stages")), sorted(smoke));
    let tables = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../.config/stages.toml"
    ));
    assert!(justfile_table(tables, "stage_inputs").is_some());
}

#[test]
fn should_refuse_before_the_first_stage_without_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let ran = dir.path().join("ran");
    let lock = crate::locks::Lock::of(&|name| {
        (name == "MACHINE_LOCK").then(|| "/nonexistent/with-machine-lock".to_string())
    });
    std::env::set_var(
        "DEV_TOOLS_JUST",
        script(dir.path(), "just", &format!("touch {}", ran.display())),
    );
    let refused = super::run(&["fmt".into()], lock).unwrap_err();
    assert!(refused.contains(crate::locks::INSTALL), "{refused}");
    assert!(!ran.exists(), "a stage ran before the refusal");
}
