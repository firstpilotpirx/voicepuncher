//! The journal line as the former Perl runner wrote it for a fixture report and log (golden lines), and the
//! summary's overlap report.

use super::report::Report;
use super::{line, times, Run};
use jiff::tz::TimeZone;
use std::path::{Path, PathBuf};

const LOG: &str =
    "\x1b[31m        FAIL\x1b[0m [   1.234s] (3/10) voicepuncher::it placed_commands::should_spot
     TIMEOUT [ 120.000s] voicepuncher::it slow::should_hang
     SIGTERM [   2.000s] voicepuncher::it other::should_stop
  Scenario: A paste lands
   ✘  Then the text is pasted
  Scenario Outline: Second été
   ✘ step
        FAIL [   1.234s] (3/10) voicepuncher::it placed_commands::should_spot
";

const PLAIN: &str = r#"{"started":"1970-01-01T00:16:30+00:00","seconds":120.2,"worktree":"/nonexistent/worktree","branch":null,"commit":null,"recipe":"check","result":"failed","exit":1,"stages":[{"name":"fmt","result":"passed","seconds":12.3},{"from":{"branch":"feat/x","commit":"abc1234","finished":"2026-10-23T10:00:00+02:00","seconds":55.5,"worktree":"/w"},"name":"test","result":"reused","seconds":null},{"name":"build","result":"passed","seconds":40},{"name":"it-real","result":"skipped","seconds":0.4},{"name":"e2e-smoke-voice","result":"failed","seconds":61.75},{"name":"e2e-cues","result":"skipped","seconds":null}],"lock_wait":5.2,"lock_held":94.8,"locks":[{"class":"e2e","held":94.8,"locked":"1970-01-01T00:16:45+00:00","wait":5.2}],"accuracy":"accuracy: 10 recordings (e2e-quick) — x","failed_tests":["voicepuncher::it placed_commands::should_spot","voicepuncher::it slow::should_hang","voicepuncher::it other::should_stop","scenario: A paste lands","scenario: Second été"]}"#;

const CALLER: &str = r#"{"started":"1970-01-01T03:00:05+03:00","seconds":40,"worktree":"/nonexistent/worktree","branch":null,"commit":null,"recipe":"stages","result":"passed","exit":0,"stages":[{"name":"e2e-quick","result":"passed","seconds":30}],"lock_wait":"caller","lock_held":"caller","locks":[],"accuracy":null,"failed_tests":[]}"#;

fn fixture(dir: &Path) -> (Report, Run) {
    let pass = dir.join("pass.json");
    std::fs::write(&pass, r#"{"branch":"feat/x","commit":"abc1234","dirty":false,"finished":"2026-10-23T10:00:00+02:00","key":"k","seconds":55.5,"stage":"test","worktree":"/w"}"#).unwrap();
    std::fs::write(dir.join("log"), LOG).unwrap();
    let report = format!(
        "stage ✓ fmt 12.3\nreused test {}\nstage ✓ build 40\nrequested e2e 1000.0\nlocked e2e 1005.25\nstage – it-real 0.4\nstage ✗ e2e-smoke-voice 61.75\nreturned e2e 1100.0\n",
        pass.display()
    );
    let run = Run {
        log: dir.join("log"),
        stages: [
            "fmt",
            "test",
            "build",
            "it-real",
            "e2e-smoke-voice",
            "e2e-cues",
        ]
        .map(String::from)
        .to_vec(),
        started: 990.05,
        ended: 1110.3,
        exit: 1,
        interrupted: false,
        caller: false,
        worktree: PathBuf::from("/nonexistent/worktree"),
        recipe: Some("check".into()),
        accuracy: Some("accuracy: 10 recordings (e2e-quick) — x".into()),
    };
    (Report::parse(&report), run)
}

#[test]
fn should_write_the_journal_line_the_former_runner_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let (report, run) = fixture(dir.path());
    assert_eq!(line(&run, &report, &TimeZone::UTC), PLAIN);
}

#[test]
fn should_leave_the_tests_an_interrupt_stopped_out_of_the_failures() {
    let dir = tempfile::tempdir().unwrap();
    let (report, mut run) = fixture(dir.path());
    run.exit = 130;
    run.interrupted = true;
    let expected = PLAIN
        .replace(
            r#""result":"failed","exit":1"#,
            r#""result":"interrupted","exit":130"#,
        )
        .replace(r#""voicepuncher::it other::should_stop","#, "");
    assert_eq!(line(&run, &report, &TimeZone::UTC), expected);
}

#[test]
fn should_name_the_caller_when_it_held_the_machine_lock() {
    let dir = tempfile::tempdir().unwrap();
    let report = Report::parse(
        "stage ✓ e2e-quick 30\nrequested machine 10\nlocked machine 10\nreturned machine 40\n",
    );
    let run = Run {
        log: dir.path().join("missing"),
        stages: vec!["e2e-quick".into()],
        started: 5.0,
        ended: 45.0,
        exit: 0,
        interrupted: false,
        caller: true,
        worktree: PathBuf::from("/nonexistent/worktree"),
        recipe: None,
        accuracy: None,
    };
    let moscow = TimeZone::get("Europe/Moscow").unwrap();
    assert_eq!(line(&run, &report, &moscow), CALLER);
}

#[test]
fn should_record_a_lock_that_was_never_taken_without_a_hold() {
    let dir = tempfile::tempdir().unwrap();
    let report = Report::parse("requested e2e 1000.0\nreturned e2e 1010.0\n");
    let (_, mut run) = fixture(dir.path());
    run.stages = vec!["e2e-quick".into()];
    let text = line(&run, &report, &TimeZone::UTC);
    assert!(
        text.contains(
            r#""lock_wait":10,"lock_held":null,"locks":[{"class":"e2e","held":null,"wait":10}]"#
        ),
        "{text}"
    );
}

fn journal_run(started: &str, branch: &str, locked: &str, held: u32) -> String {
    format!(
        r#"{{"started":"{started}","branch":"{branch}","recipe":"check","result":"passed","seconds":700,"stages":[],"failed_tests":[],"locks":[{{"class":"e2e","locked":"{locked}","held":{held},"wait":1}}]}}"#
    )
}

#[test]
fn should_name_the_runs_whose_lock_holds_overlapped() {
    let journal = [
        journal_run(
            "2026-10-23T10:00:00+02:00",
            "fix/a",
            "2026-10-23T10:01:00+02:00",
            600,
        ),
        journal_run(
            "2026-10-23T10:05:00+02:00",
            "fix/b",
            "2026-10-23T10:09:00+02:00",
            300,
        ),
        journal_run(
            "2026-10-23T10:20:00+02:00",
            "fix/c",
            "2026-10-23T10:21:00+02:00",
            300,
        ),
    ];
    let out = times::summary(&journal.join("\n"), "2020-01-01", 0.0, &TimeZone::UTC, "j").unwrap();
    assert!(
        out.contains("fix/a 2026-10-23T10:00:00+02:00 ↔ fix/b 2026-10-23T10:05:00+02:00: 2m 00s"),
        "{out}"
    );
    assert!(!out.contains("fix/c 2026-10-23T10:20"), "{out}");
    let turns = [journal[0].clone(), journal[2].clone()].join("\n");
    let out = times::summary(&turns, "2020-01-01", 0.0, &TimeZone::UTC, "j").unwrap();
    assert!(out.contains("e2e lock overlaps: none (2 holds)"), "{out}");
}

#[test]
fn should_summarize_the_stages_branches_and_failed_tests() {
    let journal = PLAIN.replace("1970-01-01T00:16:30", "2026-10-23T10:00:00");
    let out = times::summary(&journal, "2026-10-01", 0.0, &TimeZone::UTC, "j").unwrap();
    let expected = "stage times since 2026-10-01 00:00: 1 runs — 1 failed

recipe           stage                 runs    median      total  failed  reused      saved
check            fmt                      1     12.3s      12.3s       0       0          -
check            test                     0         -       0.0s       0       1      55.5s
check            build                    1     40.0s      40.0s       0       0          -
check            e2e-smoke-voice          1    1m 01s     1m 01s       1       0          -

reuse saved: 1 stages, 55.5s

lock wait: 1 runs, median 5.2s, total 5.2s

e2e lock overlaps: none (1 holds)

failed tests:
     1  scenario: A paste lands
     1  scenario: Second été
     1  voicepuncher::it other::should_stop
     1  voicepuncher::it placed_commands::should_spot
     1  voicepuncher::it slow::should_hang

runs per branch:
  (none)                                      1 runs    1 failed     2m 00s
";
    assert_eq!(out, expected);
}

#[test]
fn should_refuse_a_start_it_cannot_read() {
    let refused = times::summary("", "yesterday", 0.0, &TimeZone::UTC, "j").unwrap_err();
    assert_eq!(
        refused,
        "since 'yesterday': give a span like 7d or a date like 2026-10-01"
    );
    let empty = times::summary("", "2026-10-01", 0.0, &TimeZone::UTC, "/j").unwrap();
    assert_eq!(empty, "no runs since 2026-10-01 00:00 in /j\n");
}
