//! Steps of a session's lines in the log file and on the terminal.

use super::{
    ensure, ensure_session_ran, given, header_start, new_run, run, then, when, Context,
    IntegrationWorld, Regex, Result, LINE_FORMAT,
};

#[given("the logs folder does not exist yet")]
pub(super) fn logs_folder_missing(world: &mut IntegrationWorld) {
    world.app.logging = Some(new_run());
}

#[given("the log file holds the lines of an earlier session")]
pub(super) fn earlier_session(world: &mut IntegrationWorld) {
    world.app.logging = Some(new_run());
    let run = run(world);
    run.start_app(false).expect("the earlier session runs");
    run.earlier_log = run.log_text();
}

#[when("the app starts in hotkey mode without a terminal")]
pub(super) fn start_without_terminal(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_app(false)
}

#[when("the app starts in hotkey mode on a terminal")]
pub(super) fn start_on_terminal(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_app(true)
}

#[when("the app transcribes an audio file in file mode")]
pub(super) fn transcribe_file(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let missing_audio = run.home.path().join("missing.wav");
    let mut command = run.command(env!("CARGO_BIN_EXE_voicepuncher"));
    command.arg("--config").arg(&run.config);
    command.arg("--input-file").arg(missing_audio);
    run.output = Some(command.output().context("the app runs")?);
    Ok(())
}

#[then(
    "the logs folder exists and the log file holds the session's lines in the terminal line format"
)]
fn lines_in_file(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let session = ensure_session_ran(run)?;
    ensure!(run.logs().is_dir(), "no logs folder");
    let format = Regex::new(LINE_FORMAT)?;
    for line in session.lines() {
        ensure!(format.is_match(line), "not in the line format: {line}");
    }
    Ok(())
}

#[then("the earlier lines are still in the log file and the new session's lines follow them")]
pub(super) fn appended(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(
        !run.earlier_log.is_empty(),
        "the earlier session logged nothing"
    );
    ensure!(
        run.log_text().starts_with(&run.earlier_log),
        "earlier lines gone"
    );
    ensure_session_ran(run).map(|_| ())
}

#[then("the session's lines appear on the terminal and in the log file")]
pub(super) fn on_terminal_and_file(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let session = ensure_session_ran(run)?;
    let terminal = String::from_utf8_lossy(&run.output().stdout).replace("\r\n", "\n");
    for line in session.lines() {
        ensure!(
            terminal.contains(line),
            "{line} missing on the terminal: {terminal}"
        );
    }
    Ok(())
}

#[then("the session's lines are in the log file and nothing is written to the error output")]
pub(super) fn file_only(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure_session_ran(run)?;
    ensure!(run.stderr().is_empty(), "error output: {}", run.stderr());
    Ok(())
}

#[then("its log lines are on the error output and the logs folder still does not exist")]
pub(super) fn file_mode_stderr_only(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let stderr = run.stderr();
    let first = stderr.lines().next().unwrap_or_default();
    ensure!(first.contains(&header_start()), "first line: {first}");
    ensure!(stderr.contains(" ERROR "), "error output: {stderr}");
    ensure!(
        !run.home.path().join(".voicepuncher").exists(),
        "the app made its folder"
    );
    Ok(())
}

#[then(
    "the first line of the session names the version, the process id, the config path, the macOS version and the architecture"
)]
fn header_line(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let session = ensure_session_ran(run)?;
    let first = session.lines().next().unwrap_or_default();
    let header = format!(
        r"^\[[^\]]+\] {}\d+, config {}, macOS \d+\.\d+\.\d+, {}\)$",
        regex::escape(&header_start()),
        regex::escape(&run.config.display().to_string()),
        std::env::consts::ARCH
    );
    ensure!(Regex::new(&header)?.is_match(first), "first line: {first}");
    Ok(())
}
