//! Runs the app binary with a home folder of its own and reads its log file and outputs;
//! drives the log file writer directly for rotation.

use std::io::Write;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use regex::Regex;
use tempfile::TempDir;

use voicepuncher::adapters::secondary::logging::file::{
    logs_dir, RotatingFileWriter, KEPT_BACKUPS, LOG_FILE_NAME,
};

use crate::IntegrationWorld;

pub(crate) mod panic;

mod rotation;
mod session;

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
        .app
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
