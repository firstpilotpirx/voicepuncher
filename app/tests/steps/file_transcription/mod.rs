//! Runs the app's binary from its command line in a home of its own, with nothing on its
//! standard input and every service address on a port where nothing listens, and reads its
//! exit code and outputs; the options are read from the command line's definition.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use anyhow::{ensure, Context, Result};
use clap::CommandFactory;
use cucumber::{given, then, when};
use regex::Regex;
use tempfile::TempDir;

use voicepuncher::adapters::primary::cli::Cli;

use crate::IntegrationWorld;

/// The options developers and tests need, and nothing else.
const DEVELOPER_OPTIONS: [&str; 11] = [
    "--archive-dir",
    "--config",
    "--download-models",
    "--help",
    "--hotkey",
    "--input-file",
    "--models-dir",
    "--no-archive",
    "--transcription-url",
    "--version",
    "--voice-control",
];
/// The options that change no setting, so their help says nothing of overriding.
const NOT_SETTINGS: [&str; 5] = ["config", "input-file", "download-models", "help", "version"];
const OVERRIDES: &str = "overrides the configuration for this run";
/// The former option that turned text refinement off.
const TEXT_REFINEMENT_OFF: &str = "--no-llm";
const UNKNOWN_OPTION: &str = "--no-such-option";

/// A command line run: its home, the recording it transcribes, the option it was given and
/// what the process did.
#[derive(Debug)]
pub struct CommandLineRun {
    home: TempDir,
    recording: Option<PathBuf>,
    option: String,
    output: Option<Output>,
}

fn command_line(world: &mut IntegrationWorld) -> Result<&mut CommandLineRun> {
    if world.extra.insertion.command_line.is_none() {
        let home = tempfile::tempdir()?;
        let run = CommandLineRun { home, recording: None, option: String::new(), output: None };
        world.extra.insertion.command_line = Some(run);
    }
    world
        .extra
        .insertion
        .command_line
        .as_mut()
        .context("no command line run")
}

impl CommandLineRun {
    fn start(&mut self, args: &[&std::ffi::OsStr]) -> Result<()> {
        let output = Command::new(env!("CARGO_BIN_EXE_voicepuncher"))
            .args(args)
            .env("HOME", self.home.path())
            .stdin(Stdio::null())
            .output()?;
        self.output = Some(output);
        Ok(())
    }

    fn start_with_option(&mut self, option: &str) -> Result<()> {
        option.clone_into(&mut self.option);
        self.start(&[option.as_ref()])
    }

    fn output(&self) -> Result<&Output> {
        self.output.as_ref().context("the app did not run")
    }

    fn ended_with(&self, code: i32) -> Result<String> {
        let output = self.output()?;
        let printed = String::from_utf8_lossy(&output.stderr).into_owned();
        ensure!(
            output.status.code() == Some(code),
            "exit {:?}: {printed}",
            output.status.code()
        );
        Ok(printed)
    }
}

/// An address on this machine where nothing listens.
fn refused_url() -> Result<String> {
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    Ok(format!("http://127.0.0.1:{port}/v1"))
}

/// Writes one second of silence as a 16 kHz mono WAV file.
fn write_silence(path: &std::path::Path) -> Result<()> {
    let format = hound::WavSpec {
        sample_format: hound::SampleFormat::Int,
        bits_per_sample: 16,
        sample_rate: 16_000,
        channels: 1,
    };
    let mut wav = hound::WavWriter::create(path, format)?;
    (0..format.sample_rate).try_for_each(|_| wav.write_sample(0i16))?;
    Ok(wav.finalize()?)
}

#[when("the user asks the app for its help")]
fn asks_for_help(world: &mut IntegrationWorld) -> Result<()> {
    command_line(world)?.start(&["--help".as_ref()])
}

#[then("the help lists exactly the developer options")]
fn lists_developer_options(world: &mut IntegrationWorld) -> Result<()> {
    let run = command_line(world)?;
    let output = run.output()?;
    ensure!(output.status.success(), "exit {:?}", output.status.code());
    let help = String::from_utf8_lossy(&output.stdout);
    let listed: BTreeSet<&str> = Regex::new(r"--[a-z][a-z-]*")?
        .find_iter(&help)
        .map(|found| found.as_str())
        .collect();
    ensure!(
        listed == BTreeSet::from(DEVELOPER_OPTIONS),
        "the help lists {listed:?}"
    );
    Ok(())
}

#[then("every option that changes a setting says it overrides the configuration for this run")]
fn settings_say_override(_world: &mut IntegrationWorld) -> Result<()> {
    let mut command = Cli::command();
    command.build();
    let silent: Vec<String> = command
        .get_arguments()
        .filter(|arg| {
            !arg.get_long()
                .is_some_and(|long| NOT_SETTINGS.contains(&long))
        })
        .filter(|arg| {
            !arg.get_help()
                .is_some_and(|help| help.to_string().contains(OVERRIDES))
        })
        .map(|arg| arg.get_id().to_string())
        .collect();
    ensure!(
        silent.is_empty(),
        "help says nothing of overriding: {silent:?}"
    );
    Ok(())
}

#[when("the user starts the app with the former option that turned text refinement off")]
fn starts_with_text_refinement_option(world: &mut IntegrationWorld) -> Result<()> {
    command_line(world)?.start_with_option(TEXT_REFINEMENT_OFF)
}

#[when("the user starts the app with an option it does not know")]
fn starts_with_unknown_option(world: &mut IntegrationWorld) -> Result<()> {
    command_line(world)?.start_with_option(UNKNOWN_OPTION)
}

#[then("the process ends with exit code 2 and names the option")]
fn exit_2_naming_option(world: &mut IntegrationWorld) -> Result<()> {
    let run = command_line(world)?;
    let printed = run.ended_with(2)?;
    ensure!(printed.contains(&run.option), "{printed}");
    Ok(())
}

#[given("no speech recognition service answers at the configured address")]
fn no_service(world: &mut IntegrationWorld) -> Result<()> {
    let run = command_line(world)?;
    let recording = run.home.path().join("silence.wav");
    write_silence(&recording)?;
    run.recording = Some(recording);
    Ok(())
}

#[given("a recording file that is not a WAV file")]
fn not_a_wav_file(world: &mut IntegrationWorld) -> Result<()> {
    let run = command_line(world)?;
    let recording = run.home.path().join("not-a-recording.wav");
    std::fs::write(&recording, "not a recording")?;
    run.recording = Some(recording);
    Ok(())
}

#[when("the user transcribes the recording in file mode")]
fn transcribes_in_file_mode(world: &mut IntegrationWorld) -> Result<()> {
    let run = command_line(world)?;
    let recording = run.recording.clone().context("no recording")?;
    let url = refused_url()?;
    let args = [
        "--config".as_ref(),
        "/dev/null".as_ref(),
        "--input-file".as_ref(),
        recording.as_os_str(),
        "--transcription-url".as_ref(),
        url.as_ref(),
    ];
    run.start(&args)
}

#[then(expr = "the process ends with exit code {int}")]
fn ends_with_code(world: &mut IntegrationWorld, code: i32) -> Result<()> {
    command_line(world)?.ended_with(code).map(|_| ())
}
