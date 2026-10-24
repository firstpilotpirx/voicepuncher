//! The local services the app talks to over loopback (the `justfile`'s `start-model-servers`,
//! `stop-model-servers` and the E2E pair): the speech recognition services for dictation and for commands, each
//! as one `whisper-server` or as copies 10000 above its port, the text refinement service and the command
//! spotting service. Each runs under its supervisor (`dev-tools supervise`), which starts it again when it
//! exits; the PID files in `~/.voicepuncher/run` name the supervisors, the logs are in `~/.voicepuncher/logs`.

pub mod command_lines;
pub mod e2e;
mod launch;
mod ready;
mod start;
mod stop;
pub mod supervise;
#[cfg(test)]
mod tests;

pub use e2e::command as e2e_command;
pub use start::command as start_command;
pub use stop::command as stop_command;
pub use supervise::command as supervise_command;

use crate::context::{home, var, Context};
use crate::Outcome;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The most copies a speech recognition service runs: its clients look for its copies on the four ports
/// 10001–10004 above its own (10000 below when they would pass 65535), crate copy-pool, `address.rs` `MAX_COPIES`.
pub const MAX_COPIES: u32 = 4;

/// Every service, in the order a start without names starts them: the service for commands before the command
/// spotting service, so its first confirmation finds it ready.
pub const SERVICES: [&str; 4] = [
    "whisper-server",
    "whisper-server-commands",
    "llama-server",
    "voicepuncher-command-spotting",
];

/// Why a start or stop ends early: a message printed after `✗ `, or an exit code whose reason was printed already.
#[derive(Debug, PartialEq, Eq)]
pub enum Stop {
    Message(String),
    Code(u8),
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

/// A step of a start or stop.
pub type Step = Result<(), Stop>;

/// A step as a command's outcome.
pub fn outcome(step: Step) -> Outcome {
    match step {
        Ok(()) => Ok(0),
        Err(Stop::Message(message)) => Err(message),
        Err(Stop::Code(code)) => Ok(code),
    }
}

/// One speech recognition service: its name, port, copies (as set, checked when it starts), the variable naming
/// its model and how its messages call it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Speech {
    pub name: &'static str,
    pub port: u32,
    pub copies: String,
    pub model_var: &'static str,
    pub label: &'static str,
}

/// What the `justfile` computed for its service recipes, from the same variables and defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The `justfile`'s directory: the warm-up utterance and the command spotting service's build are there.
    pub root: PathBuf,
    pub run_dir: PathBuf,
    pub log_dir: PathBuf,
    pub dictation: Speech,
    pub commands: Speech,
    pub llm_port: u32,
    pub kws_port: u32,
}

impl Settings {
    /// The settings of this environment: `VOICEPUNCHER_STT_PORT` (8081), `VOICEPUNCHER_STT_INSTANCES` (3),
    /// `VOICEPUNCHER_COMMAND_STT_PORT` (8085), `VOICEPUNCHER_COMMAND_STT_INSTANCES` (3), `VOICEPUNCHER_LLM_PORT`
    /// (8080) and `VOICEPUNCHER_KWS_PORT` (8082).
    pub fn from_env() -> Result<Self, String> {
        let home = home();
        Ok(Self {
            root: Context::from_env().root,
            run_dir: home.join(".voicepuncher/run"),
            log_dir: home.join(".voicepuncher/logs"),
            dictation: Speech {
                name: "whisper-server",
                port: port("VOICEPUNCHER_STT_PORT", "8081")?,
                copies: setting("VOICEPUNCHER_STT_INSTANCES", "3"),
                model_var: "VOICEPUNCHER_WHISPER_MODEL",
                label: "speech recognition service for dictation",
            },
            commands: Speech {
                name: "whisper-server-commands",
                port: port("VOICEPUNCHER_COMMAND_STT_PORT", "8085")?,
                copies: setting("VOICEPUNCHER_COMMAND_STT_INSTANCES", "3"),
                model_var: "VOICEPUNCHER_COMMAND_WHISPER_MODEL",
                label: "speech recognition service for commands",
            },
            llm_port: port("VOICEPUNCHER_LLM_PORT", "8080")?,
            kws_port: port("VOICEPUNCHER_KWS_PORT", "8082")?,
        })
    }

    /// The URL of the text refinement service.
    pub fn llm_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.llm_port)
    }

    /// The URL of the speech recognition service for commands, which the command spotting service confirms with.
    pub fn command_stt_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.commands.port)
    }

    /// The URL of the command spotting service.
    pub fn kws_url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.kws_port)
    }

    /// The command spotting service's program, as `cargo build --release` leaves it.
    pub fn kws_binary(&self) -> PathBuf {
        self.root
            .join("target/release/voicepuncher-command-spotting")
    }

    /// The PID file of a service's supervisor.
    pub fn pid_file(&self, binary: &str, port: u32) -> PathBuf {
        self.run_dir.join(format!("{binary}-{port}.pid"))
    }

    /// The log of a service and its supervisor.
    pub fn log_file(&self, binary: &str, port: u32) -> PathBuf {
        self.log_dir.join(format!("{binary}-{port}.log"))
    }

    /// The list of copy ports of the speech recognition service on that port, while it runs as copies.
    pub fn copies_file(&self, port: u32) -> PathBuf {
        self.run_dir.join(format!("whisper-server-{port}.copies"))
    }
}

/// A variable's value, or its default.
fn setting(name: &str, default: &str) -> String {
    var(name).unwrap_or_else(|| default.to_string())
}

/// A port variable's value, or its default.
fn port(name: &str, default: &str) -> Result<u32, String> {
    let value = setting(name, default);
    value
        .parse()
        .map_err(|_| format!("{name}: '{value}' is not a port"))
}

/// The ports of the first `copies` copies of the speech recognition service on `port`, the ports its clients look
/// at (crate copy-pool): 10001–10004 above its port, or 10000 below when those would pass 65535.
pub fn copy_ports(port: u32, copies: u32) -> Vec<u32> {
    let base = if port + 10_000 + MAX_COPIES <= 65_535 {
        port + 10_000
    } else {
        port.saturating_sub(10_000)
    };
    (1..=copies).map(|i| base + i).collect()
}

/// The services a command names, or every one.
pub fn selected(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        SERVICES.iter().map(|name| (*name).to_string()).collect()
    } else {
        args.to_vec()
    }
}

/// The message for a name that is no service.
pub fn unknown(service: &str) -> Stop {
    Stop::Message(format!(
        "unknown service '{service}' — use whisper-server, whisper-server-commands, llama-server or \
         voicepuncher-command-spotting"
    ))
}

/// The last component of a path, as `basename` prints it.
pub fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// A file's text without its trailing newlines, as `$(cat file)` gives it, or nothing when it cannot be read.
pub fn file_text(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.trim_end_matches('\n').to_string())
}

/// Writes one line of a command's output.
pub fn say(out: &mut dyn Write, line: &str) {
    // A closed output must not stop the start or stop of a service
    let _ = writeln!(out, "{line}");
}

/// Sends a signal to a process.
pub fn signal(pid: u32, signal: rustix::process::Signal) {
    if let Some(pid) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    {
        // A process that is gone already needs no signal
        let _ = rustix::process::kill_process(pid, signal);
    }
}

/// Whether a process runs (or is a zombie its parent has not reaped).
pub fn alive(pid: u32) -> bool {
    i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
        .is_some_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}
