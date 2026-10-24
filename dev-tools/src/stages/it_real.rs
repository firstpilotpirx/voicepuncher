//! `dev-tools it-real`: the `it-real` stage — the desktop tests (`needs_desktop`) on the real microphone, the
//! Accessibility API and windows on screen, never the ones that press keys in the focused app. Without a
//! permission the tests need (Microphone, Accessibility, Input Monitoring) for the app running them, it names
//! each one missing, counts the tests it leaves out and skips (exit 77). `VOICEPUNCHER_TCC_PROBE` replaces the
//! probe.

use crate::context::{var, Context};
use crate::filters::gates;
use crate::process::{code, status};
use crate::Outcome;
use std::path::PathBuf;
use std::process::Command;

const TARGETS: [&str; 6] = [
    "-p",
    "voicepuncher",
    "--test",
    "it",
    "--test",
    "main_thread",
];
pub const FILTER: &str = "test(/::needs_desktop::/) & not test(/::needs_real_keyboard::/)";
/// The exit code of a stage that skipped itself.
pub const SKIPPED: u8 = 77;

/// The filterset, narrowed to a rerun's selection and without the frozen tests.
pub fn filter(rerun: Option<&str>, frozen: Option<&str>) -> String {
    let mut filter = FILTER.to_string();
    if let Some(rerun) = rerun {
        filter = format!("{filter} & ({rerun})");
    }
    if let Some(frozen) = frozen {
        filter = format!("{filter} & not ({frozen})");
    }
    filter
}

/// The lines of the permissions the probe found missing, each naming the app to grant it to.
pub fn missing_lines(missing: &str) -> Vec<String> {
    missing
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| format!("⊘ {line}: the desktop tests are skipped (grant it in System Settings › Privacy & Security, or run just it-real by hand)"))
        .collect()
}

/// The probe: the one `VOICEPUNCHER_TCC_PROBE` names, else e2e-test's `tcc-probe`, built first.
fn probe() -> Result<PathBuf, String> {
    if let Some(probe) = var("VOICEPUNCHER_TCC_PROBE") {
        return Ok(PathBuf::from(probe));
    }
    let built = status(Command::new("cargo").args([
        "build",
        "-q",
        "-p",
        "voicepuncher-e2e-test",
        "--bin",
        "tcc-probe",
    ]));
    if built != 0 {
        return Err("the permission probe tcc-probe did not build".into());
    }
    Ok(crate::context::target_dir().join("debug/tcc-probe"))
}

pub fn command(_: &[String]) -> Outcome {
    let context = Context::from_env();
    let filter = filter(
        var("VOICEPUNCHER_RERUN_FILTER").as_deref(),
        var("VOICEPUNCHER_FROZEN_FILTER").as_deref(),
    );
    let probe = probe()?;
    let output = Command::new(&probe)
        .output()
        .map_err(|e| format!("{}: {e}", probe.display()))?;
    if !output.status.success() {
        for line in missing_lines(&String::from_utf8_lossy(&output.stdout)) {
            println!("{line}");
        }
        // The count of the desktop tests left out, as the gates print it for a closed need
        let closed = |name: &str| {
            if name == "VOICEPUNCHER_DESKTOP_IT" {
                None
            } else {
                var(name)
            }
        };
        gates::gated_in(&context, &TARGETS.join(" "), &filter, &closed)?;
        return Ok(SKIPPED);
    }
    let mut run = Command::new("cargo");
    run.args(["nextest", "run"])
        .args(TARGETS)
        .args(["-E", &filter])
        .env("VOICEPUNCHER_DESKTOP_IT", "1");
    if var("VOICEPUNCHER_FAIL_FAST").is_some() {
        run.arg("--fail-fast");
    }
    Ok(code(status(&mut run)))
}
