//! Runs the app binary with a home folder of its own and reads its log file and outputs;
//! drives the log file writer directly for rotation.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};
use regex::Regex;
use tempfile::TempDir;

use voicepuncher::adapters::secondary::logging::file::{
    logs_dir, RotatingFileWriter, KEPT_BACKUPS, LOG_FILE_NAME,
};

use crate::IntegrationWorld;

pub(crate) mod panic;

const SERVICE_UNREACHABLE_EXIT: i32 = 2;
/// `[time LEVEL thread target] message`, the line format of a terminal.
const LINE_FORMAT: &str = r"^\[\S+ (ERROR|WARN |INFO |DEBUG|TRACE) \S+ \S+\] \S";
const SMALL_LIMIT: u64 = 100;

pub struct LoggingRun {
    home: TempDir,
    config: PathBuf,
    earlier_log: String,
    output: Option<Output>,
    on_terminal: bool,
    writer: Option<RotatingFileWriter>,
    last_line: Vec<u8>,
}

impl std::fmt::Debug for LoggingRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingRun")
            .field("home", &self.home.path())
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

fn run(world: &mut IntegrationWorld) -> &mut LoggingRun {
    world
        .logging
        .as_mut()
        .expect("a Given step sets up the run")
}

fn unused_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
    listener.local_addr().expect("a bound port").port()
}

fn new_run() -> LoggingRun {
    let home = tempfile::tempdir().expect("a temporary home");
    let port = unused_port();
    let config = home.path().join("config.toml");
    let text = format!(
        "[transcription]\nbase_url = \"http://127.0.0.1:{port}/v1\"\n\
         [voice_control]\nenabled = true\nurl = \"ws://127.0.0.1:{port}\"\n\
         [services]\nmanage = false\n"
    );
    std::fs::write(&config, text).expect("the config is written");
    LoggingRun {
        home,
        config,
        earlier_log: String::new(),
        output: None,
        on_terminal: false,
        writer: None,
        last_line: Vec::new(),
    }
}

impl LoggingRun {
    fn logs(&self) -> PathBuf {
        logs_dir(self.home.path())
    }

    fn log_path(&self) -> PathBuf {
        self.logs().join(LOG_FILE_NAME)
    }

    fn log_text(&self) -> String {
        std::fs::read_to_string(self.log_path()).unwrap_or_default()
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.env("HOME", self.home.path()).stdin(Stdio::null());
        command
    }

    fn start_app(&mut self, on_terminal: bool) -> Result<()> {
        let app = env!("CARGO_BIN_EXE_voicepuncher");
        let mut command = if on_terminal {
            let mut script = self.command("/usr/bin/script");
            script.args(["-q", "/dev/null", app]);
            script
        } else {
            self.command(app)
        };
        command.arg("--config").arg(&self.config);
        self.output = Some(command.output().context("the app runs")?);
        self.on_terminal = on_terminal;
        Ok(())
    }

    fn output(&self) -> &Output {
        self.output.as_ref().expect("a When step ran the app")
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output().stderr).into_owned()
    }

    fn session_log(&self) -> String {
        self.log_text()[self.earlier_log.len()..].to_owned()
    }

    fn writer(&mut self) -> &mut RotatingFileWriter {
        self.writer.as_mut().expect("a writer is open")
    }

    fn open_writer(&mut self) {
        let writer = RotatingFileWriter::open(&self.log_path(), SMALL_LIMIT, KEPT_BACKUPS);
        self.writer = Some(writer.expect("the log file opens"));
    }

    fn backup(&self, number: usize) -> PathBuf {
        PathBuf::from(format!("{}.{number}", self.log_path().display()))
    }
}

fn filler(marker: u8, len: usize) -> Vec<u8> {
    let mut bytes = vec![marker; len - 1];
    bytes.push(b'\n');
    bytes
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn header_start() -> String {
    format!("voicepuncher {} starting (pid ", env!("CARGO_PKG_VERSION"))
}

fn ensure_session_ran(run: &LoggingRun) -> Result<String> {
    let code = run.output().status.code();
    if !run.on_terminal && code != Some(SERVICE_UNREACHABLE_EXIT) {
        bail!(
            "expected exit {SERVICE_UNREACHABLE_EXIT}, got {code:?}; stderr: {}",
            run.stderr()
        );
    }
    let session = run.session_log();
    ensure!(session.contains(&header_start()), "no header in: {session}");
    ensure!(session.contains(" ERROR "), "no error line in: {session}");
    Ok(session)
}

#[given("the logs folder does not exist yet")]
fn logs_folder_missing(world: &mut IntegrationWorld) {
    world.logging = Some(new_run());
}

#[given("the log file holds the lines of an earlier session")]
fn earlier_session(world: &mut IntegrationWorld) {
    world.logging = Some(new_run());
    let run = run(world);
    run.start_app(false).expect("the earlier session runs");
    run.earlier_log = run.log_text();
}

#[when("the app starts in hotkey mode without a terminal")]
fn start_without_terminal(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_app(false)
}

#[when("the app starts in hotkey mode on a terminal")]
fn start_on_terminal(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_app(true)
}

#[when("the app transcribes an audio file in file mode")]
fn transcribe_file(world: &mut IntegrationWorld) -> Result<()> {
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
fn appended(world: &mut IntegrationWorld) -> Result<()> {
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
fn on_terminal_and_file(world: &mut IntegrationWorld) -> Result<()> {
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
fn file_only(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure_session_ran(run)?;
    ensure!(run.stderr().is_empty(), "error output: {}", run.stderr());
    Ok(())
}

#[then("its log lines are on the error output and the logs folder still does not exist")]
fn file_mode_stderr_only(world: &mut IntegrationWorld) -> Result<()> {
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

#[given("a log file just below its size limit")]
fn just_below_limit(world: &mut IntegrationWorld) {
    world.logging = Some(new_run());
    let run = run(world);
    run.open_writer();
    run.writer().write_all(&filler(b'a', 90)).expect("written");
}

#[when("a line is written that would take it past the limit")]
fn write_past_limit(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.last_line = b"the line that crosses the limit\n".to_vec();
    let line = run.last_line.clone();
    Ok(run.writer().write_all(&line)?)
}

#[then("the former content is in the first backup and the new log file holds only that whole line")]
fn rotated_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(read(&run.backup(1)) == String::from_utf8(filler(b'a', 90))?);
    ensure!(read(&run.log_path()).as_bytes() == run.last_line.as_slice());
    Ok(())
}

#[given("a log file with five backups")]
fn five_backups(world: &mut IntegrationWorld) {
    world.logging = Some(new_run());
    let run = run(world);
    run.open_writer();
    for marker in b'a'..=b'f' {
        run.writer()
            .write_all(&filler(marker, 90))
            .expect("written");
    }
}

#[when("the log file rotates again")]
fn rotate_again(world: &mut IntegrationWorld) -> Result<()> {
    Ok(run(world).writer().write_all(&filler(b'g', 90))?)
}

#[then("each backup moved one number up, the former fifth backup is gone and six files remain")]
fn shifted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    for (number, marker) in (1..=KEPT_BACKUPS).zip([b'f', b'e', b'd', b'c', b'b']) {
        let expected = String::from_utf8(filler(marker, 90))?;
        ensure!(
            read(&run.backup(number)) == expected,
            "backup {number} holds the wrong part"
        );
    }
    let files = std::fs::read_dir(run.logs())?.count();
    ensure!(files == KEPT_BACKUPS + 1, "{files} files");
    Ok(())
}

#[given("a log file filled past half its limit by an earlier session")]
fn earlier_half(world: &mut IntegrationWorld) {
    world.logging = Some(new_run());
    let run = run(world);
    run.open_writer();
    run.writer().write_all(&filler(b'a', 60)).expect("written");
    run.writer = None;
}

#[when("a new session writes as much again")]
fn new_session_writes(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.open_writer();
    Ok(run.writer().write_all(&filler(b'b', 60))?)
}

#[then("the log file rotated when it passed the limit, counting what was there before the session")]
fn counted_earlier(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(read(&run.backup(1)) == String::from_utf8(filler(b'a', 60))?);
    ensure!(read(&run.log_path()) == String::from_utf8(filler(b'b', 60))?);
    Ok(())
}
