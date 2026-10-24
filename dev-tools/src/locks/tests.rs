//! The lock settings and command lines, and (`needs_machine_lock`, with agent-kit's lock) how the e2e and quiet
//! locks keep runs apart (former `_test-e2e-locks`).

use super::{Lock, DEFAULT_WAIT, INSTALL};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

fn lock_of(pairs: &[(&str, &str)]) -> Lock {
    let env: BTreeMap<String, String> = pairs
        .iter()
        .map(|(name, value)| ((*name).into(), (*value).into()))
        .collect();
    Lock::of(&|name| env.get(name).cloned())
}

fn words(line: &[OsString]) -> String {
    line.iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn should_wait_6_hours_unless_the_caller_sets_the_wait() {
    assert_eq!(lock_of(&[]).wait, DEFAULT_WAIT);
    assert_eq!(DEFAULT_WAIT, 6 * 3600);
    assert_eq!(lock_of(&[("MACHINE_LOCK_WAIT", "5")]).wait, 5);
}

#[test]
fn should_take_the_machine_lock_then_the_quiet_lock_for_the_model_stages() {
    let lock = lock_of(&[
        ("MACHINE_LOCK", "/l/lock"),
        ("MACHINE_LOCK_DIR", "/d"),
        ("MACHINE_LOCK_WAIT", "10"),
        ("MACHINE_LOCK_TIMEOUT", "60"),
    ]);
    assert_eq!(
        words(&lock.e2e(&["just".into(), "_model-stages".into()])),
        "env MACHINE_LOCK_DIR=/d MACHINE_LOCK_TIMEOUT=70 /l/lock env MACHINE_LOCK_DIR=/d-voicepuncher-quiet MACHINE_LOCK_TIMEOUT=60 /l/lock just _model-stages"
    );
    assert_eq!(
        words(&lock.quiet(&["just".into()])),
        "env MACHINE_LOCK_DIR=/d-voicepuncher-quiet /l/lock --shared just"
    );
    assert_eq!(
        words(&lock.machine(&["just".into()])),
        "env MACHINE_LOCK_DIR=/d /l/lock just"
    );
}

#[test]
fn should_run_without_the_lock_in_the_hosted_ci() {
    let lock = lock_of(&[
        ("VOICEPUNCHER_NO_MACHINE_LOCK", "1"),
        ("MACHINE_LOCK", "/elsewhere"),
    ]);
    assert_eq!(lock.wrapper, None);
    assert_eq!(words(&lock.e2e(&["just".into()])), "just");
    assert_eq!(words(&lock.quiet(&["just".into()])), "just");
    assert_eq!(words(&lock.machine(&["just".into()])), "just");
    assert!(lock.require().is_ok());
}

#[test]
fn should_use_the_agent_kit_lock_by_default() {
    let wrapper = lock_of(&[]).wrapper.unwrap();
    assert!(
        wrapper.ends_with(".claude/bin/with-machine-lock"),
        "{}",
        wrapper.display()
    );
}

#[test]
fn should_name_the_install_command_when_the_lock_is_missing() {
    let refused = lock_of(&[("MACHINE_LOCK", "/nonexistent/with-machine-lock")])
        .require()
        .unwrap_err();
    assert!(refused.contains(INSTALL), "{refused}");
    assert!(
        refused.contains("VOICEPUNCHER_NO_MACHINE_LOCK=1"),
        "{refused}"
    );
}

mod needs_machine_lock {
    use super::*;
    use std::process::{Child, Command};
    use std::time::Duration;

    /// A lock of a scratch directory, with the wrapper `MACHINE_LOCK` names.
    fn scratch_lock(dir: &std::path::Path) -> Lock {
        let wrapper = voicepuncher_test_steps::required_env("MACHINE_LOCK");
        let lock_dir = dir.join("machine-lock").to_string_lossy().into_owned();
        lock_of(&[("MACHINE_LOCK", &wrapper), ("MACHINE_LOCK_DIR", &lock_dir)])
    }

    /// Holds a lock 1.5 s, touching `<dir>/<name>.start` when it got it and `<dir>/<name>.end` when it let go.
    fn hold(dir: &std::path::Path, name: &str, line: &[OsString]) -> Child {
        let file = dir.join(name);
        let script = format!(": >'{0}.start'; sleep 1.5; : >'{0}.end'", file.display());
        let mut full = line.to_vec();
        full.extend(["sh".into(), "-c".into(), script.into()]);
        Command::new(&full[0])
            .args(&full[1..])
            .env("MACHINE_LOCK_PRIORITY", "normal")
            .spawn()
            .unwrap()
    }

    fn span(dir: &std::path::Path, name: &str) -> (f64, f64) {
        let at = |end: &str| {
            let modified = std::fs::metadata(dir.join(format!("{name}.{end}")))
                .unwrap()
                .modified()
                .unwrap();
            modified
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
        };
        (at("start"), at("end"))
    }

    fn overlap(dir: &std::path::Path, a: &str, b: &str) -> bool {
        let ((a0, a1), (b0, b1)) = (span(dir, a), span(dir, b));
        a0 < b1 && b0 < a1
    }

    fn join(children: Vec<Child>) {
        for mut child in children {
            assert!(child.wait().unwrap().success());
        }
    }

    fn pause() {
        std::thread::sleep(Duration::from_millis(500));
    }

    #[test]
    fn should_let_two_model_phases_take_turns() {
        let dir = tempfile::tempdir().unwrap();
        let lock = scratch_lock(dir.path());
        join(vec![
            hold(dir.path(), "a", &lock.e2e(&[])),
            hold(dir.path(), "b", &lock.e2e(&[])),
        ]);
        assert!(!overlap(dir.path(), "a", "b"));
    }

    #[test]
    fn should_keep_a_model_phase_out_of_an_exclusive_run() {
        let dir = tempfile::tempdir().unwrap();
        let lock = scratch_lock(dir.path());
        let machine = hold(dir.path(), "machine", &lock.machine(&[]));
        pause();
        join(vec![machine, hold(dir.path(), "e2e", &lock.e2e(&[]))]);
        assert!(!overlap(dir.path(), "machine", "e2e"));
    }

    #[test]
    fn should_let_quiet_builds_overlap() {
        let dir = tempfile::tempdir().unwrap();
        let lock = scratch_lock(dir.path());
        join(vec![
            hold(dir.path(), "a", &lock.quiet(&[])),
            hold(dir.path(), "b", &lock.quiet(&[])),
        ]);
        assert!(overlap(dir.path(), "a", "b"));
    }

    #[test]
    fn should_make_a_model_phase_wait_for_the_quiet_builds_and_keep_new_ones_out() {
        let dir = tempfile::tempdir().unwrap();
        let lock = scratch_lock(dir.path());
        let first = hold(dir.path(), "quiet1", &lock.quiet(&[]));
        pause();
        let model = hold(dir.path(), "e2e", &lock.e2e(&[]));
        pause();
        join(vec![
            first,
            model,
            hold(dir.path(), "quiet2", &lock.quiet(&[])),
        ]);
        assert!(span(dir.path(), "quiet1").1 <= span(dir.path(), "e2e").0);
        assert!(span(dir.path(), "e2e").1 <= span(dir.path(), "quiet2").0);
    }

    #[test]
    fn should_find_the_wrapper_it_runs() {
        let wrapper = PathBuf::from(voicepuncher_test_steps::required_env("MACHINE_LOCK"));
        assert!(lock_of(&[("MACHINE_LOCK", &wrapper.to_string_lossy())])
            .require()
            .is_ok());
    }
}
