//! `dev-tools start-model-servers [service…]`: starts the speech recognition services for dictation and for
//! commands and the command spotting service, or only the named ones; one that
//! already answers on its port is skipped.

use super::command_lines::{command_spotting_args, whisper_args};
use super::launch::{self, Service};
use super::ready::{port_open, Ready};
use super::{copy_ports, outcome, selected, unknown, Settings, Speech, Step, Stop, MAX_COPIES};
use crate::context::var;
use crate::process::{code, find_program, status, success_stdout};
use crate::Outcome;
use std::io::{BufRead, IsTerminal};
use std::process::Command;

/// The cargo selection of the command spotting service's build, the one the model stages build too.
const COMMAND_SPOTTING_BUILD: [&str; 2] = ["-p", "voicepuncher-command-spotting"];

/// `dev-tools start-model-servers [service…]`.
pub fn command(args: &[String]) -> Outcome {
    let settings = Settings::from_env()?;
    for dir in [&settings.run_dir, &settings.log_dir] {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    outcome(start(&settings, args))
}

/// Starts the named services, or every one, in order; the first that cannot start ends the run.
pub fn start(settings: &Settings, services: &[String]) -> Step {
    for service in selected(services) {
        match service.as_str() {
            "whisper-server" => start_whisper(settings, &settings.dictation)?,
            "whisper-server-commands" => start_whisper(settings, &settings.commands)?,
            "voicepuncher-command-spotting" => start_command_spotting(settings)?,
            _ => return Err(unknown(&service)),
        }
    }
    Ok(())
}

/// The copies a speech recognition service may run: 1 to `MAX_COPIES`.
pub fn checked_copies(speech: &Speech) -> Result<u32, Stop> {
    speech
        .copies
        .parse::<u32>()
        .ok()
        .filter(|copies| {
            (1..=MAX_COPIES).contains(copies) && speech.copies.bytes().all(|b| b.is_ascii_digit())
        })
        .ok_or_else(|| {
            Stop::Message(format!(
                "{}: {} copies — set 1 to {MAX_COPIES}: its clients look for at most {MAX_COPIES}",
                speech.label, speech.copies
            ))
        })
}

/// One speech recognition service, as one `whisper-server` on its port or as copies on the ports its clients find
/// from that port, with nothing on the port itself. Its PID, log and copy-list files are named by its port, so the
/// two services never share a file.
fn start_whisper(settings: &Settings, speech: &Speech) -> Step {
    require_binary("whisper-server", "whisper-cpp")?;
    let copies = checked_copies(speech)?;
    let port = speech.port;
    for answering in std::iter::once(port).chain(copy_ports(port, MAX_COPIES)) {
        if port_open(answering) {
            println!(
                "✓ {} already running at http://127.0.0.1:{port}/v1 (port {answering} answers)",
                speech.label
            );
            return Ok(());
        }
    }
    let model = require_model(speech.model_var)?;
    let vad = require_model("VOICEPUNCHER_VAD_MODEL")?;
    let cores = cores();
    if copies == 1 {
        // A copy list left from an earlier start would make the stop look for copies
        launch::remove(&settings.copies_file(port));
        let service = whisper_service(
            settings,
            port,
            speech.label.to_string(),
            [&model, &vad],
            cores,
        );
        return launch::start(settings, &service);
    }
    start_copies(settings, speech, copies, [&model, &vad], cores)
}

/// The copies of a speech recognition service; a copy that cannot start stops what this start already started of
/// this service only.
fn start_copies(
    settings: &Settings,
    speech: &Speech,
    copies: u32,
    models: [&str; 2],
    cores: u32,
) -> Step {
    let threads = (cores / copies).max(1);
    let ports = copy_ports(speech.port, copies);
    let list: Vec<String> = ports.iter().map(u32::to_string).collect();
    let file = settings.copies_file(speech.port);
    std::fs::write(&file, format!("{}\n", list.join(" ")))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    for (i, port) in ports.iter().enumerate() {
        let label = format!("{}: copy {} of {copies}", speech.label, i + 1);
        let started = launch::start(
            settings,
            &whisper_service(settings, *port, label, models, threads),
        );
        if started.is_err() {
            // The stop's output and its own failure stay out of the start's: the start's failure is the news
            let _ = super::stop::stop(settings, &[speech.name.to_string()], &mut std::io::sink());
            return started;
        }
    }
    println!(
        "✓ {}: {copies} copies on ports {}, found by its clients from http://127.0.0.1:{}/v1",
        speech.label,
        list.join(" "),
        speech.port
    );
    Ok(())
}

/// One `whisper-server` on a port, with its model and VAD model and that many threads.
fn whisper_service(
    settings: &Settings,
    port: u32,
    label: String,
    models: [&str; 2],
    threads: u32,
) -> Service {
    let mut command = vec!["whisper-server".to_string()];
    command.extend(whisper_args(
        models[0],
        &port.to_string(),
        &threads.to_string(),
        models[1],
    ));
    Service {
        binary: "whisper-server".into(),
        port,
        label,
        ready: Ready::Whisper { port, warm_up: settings.root.join("app/assets/warm-up.wav") },
        health: None,
        command,
    }
}

/// The command spotting service, built first.
fn start_command_spotting(settings: &Settings) -> Step {
    let port = settings.kws_port;
    if port_open(port) {
        println!(
            "✓ command spotting service already running at {}",
            settings.kws_url()
        );
        return Ok(());
    }
    let model_dir = require_model_dir("VOICEPUNCHER_KWS_MODEL_DIR")?;
    let vad = require_model("VOICEPUNCHER_KWS_VAD_MODEL")?;
    build_command_spotting(settings)?;
    let mut command = vec![settings.kws_binary().display().to_string()];
    command.extend(command_spotting_args(
        &port.to_string(),
        &model_dir,
        &vad,
        &settings.command_stt_url(),
    ));
    if let Some(dir) = var("VOICEPUNCHER_KWS_SAVE_SPEECH_DIR") {
        command.extend(["--save-speech-dir".to_string(), dir]);
    }
    let service = Service {
        binary: "voicepuncher-command-spotting".into(),
        port,
        label: "command spotting service".into(),
        ready: Ready::Port { port },
        health: Some(format!("http://127.0.0.1:{port}/health")),
        command,
    };
    launch::start(settings, &service)
}

/// Builds the command spotting service without the variables cargo sets for a test it runs, because the
/// `@local_services` and `@resilience` steps start services inside `cargo test`: ring's build script reruns when
/// `CARGO_PKG_*` or `CARGO_MANIFEST_DIR` change, so the service and its dependencies would be built again under the
/// machine lock, and once more by the next plain build.
fn build_command_spotting(settings: &Settings) -> Step {
    let mut build = Command::new("cargo");
    build
        .args(["build", "--release"])
        .args(COMMAND_SPOTTING_BUILD)
        .current_dir(&settings.root);
    for (name, _) in std::env::vars_os() {
        if dropped_by_build(&name.to_string_lossy()) {
            build.env_remove(&name);
        }
    }
    match status(&mut build) {
        0 => Ok(()),
        failed => Err(Stop::Code(code(failed))),
    }
}

/// Whether the build of the command spotting service drops a variable cargo set for a test.
pub fn dropped_by_build(name: &str) -> bool {
    name.starts_with("CARGO_MANIFEST_")
        || name.starts_with("CARGO_PKG_")
        || [
            "CARGO_CRATE_NAME",
            "CARGO_BIN_NAME",
            "CARGO_PRIMARY_PACKAGE",
            "CARGO_TARGET_TMPDIR",
            "CARGO_RUSTC_CURRENT_DIR",
        ]
        .contains(&name)
}

/// The processor cores, as `sysctl -n hw.ncpu` counts them.
fn cores() -> u32 {
    success_stdout(Command::new("sysctl").args(["-n", "hw.ncpu"]))
        .and_then(|out| out.trim().parse().ok())
        .or_else(|| {
            std::thread::available_parallelism()
                .ok()
                .and_then(|n| u32::try_from(n.get()).ok())
        })
        .unwrap_or(1)
}

/// The program is on the `PATH`; on a terminal, the user may have Homebrew install it.
fn require_binary(binary: &str, formula: &str) -> Step {
    if find_program(binary).is_some() {
        return Ok(());
    }
    if std::io::stdin().is_terminal() && ask_install(binary, formula) {
        return match status(Command::new("brew").args(["install", formula])) {
            0 => Ok(()),
            failed => Err(Stop::Code(code(failed))),
        };
    }
    Err(Stop::Message(format!(
        "{binary} not found — run 'brew install {formula}'"
    )))
}

/// Asks whether to install the formula; yes only for `y` or `Y`.
fn ask_install(binary: &str, formula: &str) -> bool {
    eprint!("{binary} not found. Install {formula} with Homebrew? [y/N] ");
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer).is_ok() && is_yes(&answer)
}

/// Whether an answer line is `y` or `Y`.
pub fn is_yes(answer: &str) -> bool {
    matches!(answer.strip_suffix('\n').unwrap_or(answer), "y" | "Y")
}

/// The model file a variable names.
pub fn require_model(variable: &str) -> Result<String, Stop> {
    let path = std::env::var(variable).unwrap_or_default();
    if path.is_empty() || !std::path::Path::new(&path).is_file() {
        return Err(Stop::Message(format!(
            "{variable}: no model file (got '{path}'). Run 'just models' or set it in .env"
        )));
    }
    Ok(path)
}

/// The model directory a variable names.
fn require_model_dir(variable: &str) -> Result<String, Stop> {
    let path = std::env::var(variable).unwrap_or_default();
    if path.is_empty() || !std::path::Path::new(&path).is_dir() {
        return Err(Stop::Message(format!(
            "{variable}: no model directory (got '{path}'). Run 'just models' or set it in .env"
        )));
    }
    Ok(path)
}
