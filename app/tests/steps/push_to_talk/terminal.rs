//! Runs the app's binary in a home of its own with nothing on its standard input, and reads
//! the hotkey listener's sources and key classification.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::adapters::primary::ptt::eventtap::keys::{
    classify, key_event, ChordState, TapKind, LEFT_OPTION, RIGHT_OPTION,
};
use voicepuncher::core::domain::{HotKey, PttSignal};

use crate::IntegrationWorld;

/// What turns key codes into typed text: the keyboard library and the input-source calls.
const TEXT_LOOKUPS: [&str; 5] = ["rdev", "TSMGet", "TISCopy", "TISGet", "UCKeyTranslate"];
/// The hotkey listener's sources.
const LISTENER: &str = "src/adapters/primary/ptt";
/// The Option flag and the device bits of each Option key.
const ALTERNATE: u64 = 0x0008_0000;
const RIGHT_DEVICE_BIT: u64 = 0x40;
const LEFT_DEVICE_BIT: u64 = 0x20;

/// A first launch in a home of its own: the home and what the app did.
#[derive(Debug, Default)]
pub struct LaunchRun {
    home: Option<TempDir>,
    output: Option<Output>,
    signals: Vec<Option<PttSignal>>,
}

fn launch(world: &mut IntegrationWorld) -> &mut LaunchRun {
    world
        .extra
        .ptt
        .launch
        .get_or_insert_with(LaunchRun::default)
}

fn run_app(home: &Path, args: &[&std::ffi::OsStr]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_voicepuncher"))
        .args(args)
        .env("HOME", home)
        .stdin(Stdio::null())
        .output()?)
}

fn config_of(run: &LaunchRun) -> Result<PathBuf> {
    Ok(run
        .home
        .as_ref()
        .context("no home")?
        .path()
        .join("settings/config.toml"))
}

#[given("there is no configuration file yet")]
fn no_configuration(world: &mut IntegrationWorld) -> Result<()> {
    launch(world).home = Some(tempfile::tempdir()?);
    Ok(())
}

#[when("the app starts with nothing on its standard input")]
fn starts_without_input(world: &mut IntegrationWorld) -> Result<()> {
    let run = launch(world);
    let home = run.home.as_ref().context("no home")?.path().to_owned();
    let recording = home.join("not-a-recording.wav");
    std::fs::write(&recording, "not a recording")?;
    let config = config_of(run)?;
    let args = [
        "--config".as_ref(),
        config.as_os_str(),
        "--input-file".as_ref(),
        recording.as_os_str(),
    ];
    run.output = Some(run_app(&home, &args)?);
    Ok(())
}

#[when("the user starts the app with `--setup`")]
fn starts_with_setup(world: &mut IntegrationWorld) -> Result<()> {
    let home = tempfile::tempdir()?;
    let output = run_app(home.path(), &["--setup".as_ref()])?;
    let run = launch(world);
    (run.home, run.output) = (Some(home), Some(output));
    Ok(())
}

#[then("it creates the configuration file and goes on without reading its standard input")]
fn created_without_asking(world: &mut IntegrationWorld) -> Result<()> {
    let run = launch(world);
    let output = run.output.as_ref().context("the app did not run")?;
    let written = std::fs::read_to_string(config_of(run)?)?;
    ensure!(
        written == voicepuncher::config::DEFAULT_FILE,
        "the file is not the defaults"
    );
    ensure!(
        output.stdout.is_empty(),
        "the app printed a question on its standard output"
    );
    Ok(())
}

#[then("the app refuses the unknown option with exit code 2")]
fn refuses_setup(world: &mut IntegrationWorld) -> Result<()> {
    let output = launch(world)
        .output
        .as_ref()
        .context("the app did not run")?;
    let printed = String::from_utf8_lossy(&output.stderr);
    ensure!(
        output.status.code() == Some(2) && printed.contains("--setup"),
        "{printed}"
    );
    Ok(())
}

#[given("the app is built")]
#[given("the hotkey is right Option")]
#[when("its hotkey listener is inspected")]
fn nothing_to_do(_world: &mut IntegrationWorld) {}

#[then("it uses no keyboard library that turns key codes into typed text, and no call to the input-source service")]
fn reads_key_codes_only(_world: &mut IntegrationWorld) -> Result<()> {
    let found: Vec<String> = sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join(LISTENER))?
        .into_iter()
        .filter_map(|file| {
            let text = std::fs::read_to_string(&file).ok()?;
            TEXT_LOOKUPS
                .iter()
                .any(|call| text.contains(call))
                .then(|| file.display().to_string())
        })
        .collect();
    ensure!(found.is_empty(), "text lookups in {found:?}");
    Ok(())
}

fn sources(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(sources(&path)?);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

#[when("right Option goes down and up while left Option stays up")]
fn right_option_down_up(world: &mut IntegrationWorld) {
    let flags = [
        (RIGHT_OPTION, ALTERNATE | RIGHT_DEVICE_BIT),
        (RIGHT_OPTION, 0),
    ];
    launch(world).signals = classified(&flags);
}

#[when("left Option goes down and up")]
fn left_option_down_up(world: &mut IntegrationWorld) {
    let flags = [(LEFT_OPTION, ALTERNATE | LEFT_DEVICE_BIT), (LEFT_OPTION, 0)];
    launch(world).signals = classified(&flags);
}

fn classified(flags: &[(u16, u64)]) -> Vec<Option<PttSignal>> {
    let mut chord = ChordState::default();
    flags
        .iter()
        .filter_map(|&(code, flags)| key_event(TapKind::FlagsChanged, code, flags))
        .map(|event| classify(event, HotKey::RightOption, &mut chord))
        .collect()
}

#[then("the listener reports a press and then a release")]
fn press_then_release(world: &mut IntegrationWorld) -> Result<()> {
    let signals = &launch(world).signals;
    ensure!(
        *signals == [Some(PttSignal::Pressed), Some(PttSignal::Released)],
        "{signals:?}"
    );
    Ok(())
}

#[then("the listener reports nothing")]
fn reports_nothing(world: &mut IntegrationWorld) -> Result<()> {
    let signals = &launch(world).signals;
    ensure!(
        signals.len() == 2 && signals.iter().all(Option::is_none),
        "{signals:?}"
    );
    Ok(())
}
