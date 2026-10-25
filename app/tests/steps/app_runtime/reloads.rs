//! What a reload is seen and caused by: the observers of the runtime state, the hang-up
//! signal that reloads the configuration file, and the hotkey binding the listener matches
//! every key event against.

use std::thread::ThreadId;

use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::adapters::primary::ptt::eventtap::keys::{
    classify, target, ChordState, KeyEvent, Target, KEY_E, LEFT_COMMAND, RIGHT_OPTION,
};
use voicepuncher::config::Config;
use voicepuncher::core::domain::{HotKey, HotkeyBinding};
use voicepuncher::runtime::{signals, RuntimeState};
use voicepuncher::test_doubles::prelude::*;

use super::doubles::RecordingObserver;
use super::{config_with, run, Event, FIRST_HOTKEY, REACTION, SECOND_HOTKEY};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// The configuration file a hang-up scenario edits.
const FIRST_FILE: &str = "[hotkey]\nkey = \"RightOption\"\n";
const EDITED_FILE: &str = "[hotkey]\nkey = \"F19\"\n";
/// Not on loopback: the configuration does not validate.
const BROKEN_FILE: &str =
    "[hotkey]\nkey = \"F19\"\n[transcription]\nbase_url = \"http://192.0.2.1:8081\"\n";
const REJECTED_SETTING: &str = "transcription.base_url";

/// The configuration file of a hang-up scenario and the thread that reloads it on the signal.
#[derive(Debug)]
pub struct HangUpRun {
    folder: TempDir,
    signals_thread: ThreadId,
}

impl HangUpRun {
    fn file(&self) -> PathBuf {
        self.folder.path().join("config.toml")
    }
}

/// The hotkey binding of a rebind scenario and what the listener signalled.
#[derive(Debug)]
pub struct RebindRun {
    binding: HotkeyBinding,
    chord: ChordState,
    signals: Vec<Option<PttSignal>>,
}

impl RebindRun {
    fn new(hotkey: HotKey) -> Self {
        Self {
            binding: HotkeyBinding::new(hotkey),
            chord: ChordState::default(),
            signals: Vec::new(),
        }
    }

    fn key(&mut self, event: KeyEvent) {
        let signal = classify(event, self.binding.current(), &mut self.chord);
        self.signals.push(signal);
    }
}

/// What the command line sets over the file in these scenarios, applied again on every reload.
fn overrides(config: &mut Config) {
    config.hotkey.min_press_ms = 0;
    config.hotkey.recording_mode = RecordingMode::Toggle;
    config.voice_control.enabled = false;
}

fn states(world: &mut IntegrationWorld) -> Vec<RuntimeState> {
    run(world)
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::State(state) => Some(state),
            _ => None,
        })
        .collect()
}

fn hang_up(world: &IntegrationWorld) -> Result<&HangUpRun> {
    world
        .extra
        .runtime
        .hang_up
        .as_ref()
        .context("the runtime was not started from a file")
}

fn rebind(world: &mut IntegrationWorld) -> Result<&mut RebindRun> {
    world
        .extra
        .runtime
        .rebind
        .as_mut()
        .context("no binding was set")
}

#[when("the runtime starts, reloads a valid configuration and stops")]
fn start_reload_stop(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_ready()?;
    run.reload(config_with(SECOND_HOTKEY))?;
    run.state_becomes(
        |s| matches!(s, RuntimeState::Ready { hotkey, .. } if *hotkey == SECOND_HOTKEY),
    )?;
    run.runtime
        .as_mut()
        .context("the runtime was not started")?
        .stop();
    Ok(())
}

#[then("the observer saw Starting, Ready, Restarting, Ready, Stopped in that order")]
fn saw_every_state(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(|s| *s == RuntimeState::Stopped)?;
    let seen = states(world);
    ensure!(
        matches!(
            seen.as_slice(),
            [
                RuntimeState::Starting,
                RuntimeState::Ready { .. },
                RuntimeState::Restarting,
                RuntimeState::Ready { .. },
                RuntimeState::Stopped
            ]
        ),
        "states seen: {seen:?}"
    );
    Ok(())
}

#[when("an observer registers")]
fn observer_registers(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let observer = RecordingObserver(Arc::clone(&run.events));
    run.runtime()?.observe(Box::new(observer));
    Ok(())
}

#[then("the observer is told Ready at once")]
fn told_ready_at_once(world: &mut IntegrationWorld) -> Result<()> {
    // the first observer saw Ready once; the second is told it as it registers, not later
    let readies = states(world)
        .iter()
        .filter(|s| matches!(s, RuntimeState::Ready { .. }))
        .count();
    ensure!(readies == 2, "states seen: {:?}", states(world));
    Ok(())
}

#[given("the runtime is Ready from a configuration file")]
fn ready_from_file(world: &mut IntegrationWorld) -> Result<()> {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("config.toml");
    std::fs::write(&file, FIRST_FILE)?;
    let mut config = voicepuncher::config::load(&file)?;
    overrides(&mut config);
    let run = run(world);
    run.start_from(config)?;
    let reloader = run.reloader()?;
    let thread = signals::reload_on_hang_up(file, Box::new(overrides), reloader)?;
    let signals_thread = thread.thread().id();
    world.extra.runtime.hang_up = Some(HangUpRun { folder, signals_thread });
    Ok(())
}

#[given("the file was edited to a valid configuration with another hotkey")]
fn file_edited(world: &mut IntegrationWorld) -> Result<()> {
    Ok(std::fs::write(hang_up(world)?.file(), EDITED_FILE)?)
}

#[given("the file was edited so that it no longer validates")]
fn file_broken(world: &mut IntegrationWorld) -> Result<()> {
    Ok(std::fs::write(hang_up(world)?.file(), BROKEN_FILE)?)
}

#[when("the process receives the hang-up signal")]
fn receives_hang_up(_world: &mut IntegrationWorld) -> Result<()> {
    signal_hook::low_level::raise(signal_hook::consts::SIGHUP)?;
    Ok(())
}

#[then("the runtime reloads and the state is Ready")]
fn reloads_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(
        |s| matches!(s, RuntimeState::Ready { hotkey, .. } if *hotkey == SECOND_HOTKEY),
    )?;
    ensure!(
        run.count(|e| *e == Event::State(RuntimeState::Restarting)) == 1,
        "no restart: {:?}",
        run.events()
    );
    Ok(())
}

#[then("the pipeline uses the hotkey from the edited file")]
fn uses_edited_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        run(world).hotkey_records(SECOND_HOTKEY),
        "the edited hotkey starts no recording"
    );
    Ok(())
}

#[then("the pipeline keeps its settings")]
fn keeps_settings(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    ensure!(
        run.hotkey_records(FIRST_HOTKEY),
        "the first hotkey no longer records"
    );
    ensure!(
        run.latest_generation() == Some(1)
            && run.count(|e| *e == Event::State(RuntimeState::Restarting)) == 0,
        "the pipeline was replaced: {:?}",
        run.events()
    );
    Ok(())
}

#[then("the log names the reason the file was rejected")]
fn log_names_file_reason(world: &mut IntegrationWorld) -> Result<()> {
    let thread = hang_up(world)?.signals_thread;
    let deadline = Instant::now() + REACTION;
    loop {
        let warnings = pipeline_log().warnings.lock().unwrap().clone();
        let named = warnings.iter().any(|(by, line)| {
            *by == thread
                && line.contains("configuration rejected")
                && line.contains(REJECTED_SETTING)
        });
        if named {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("no rejection with its reason in the log: {warnings:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[given("the binding is the first hotkey")]
fn binding_first(world: &mut IntegrationWorld) {
    world.extra.runtime.rebind = Some(RebindRun::new(FIRST_HOTKEY));
}

#[given("the binding is a chord and its modifier is held")]
fn chord_half_pressed(world: &mut IntegrationWorld) -> Result<()> {
    world.extra.runtime.rebind = Some(RebindRun::new(HotKey::CmdE));
    rebind(world)?.key(KeyEvent::Down(LEFT_COMMAND));
    Ok(())
}

#[when("the binding changes to a second hotkey and the second hotkey is pressed and released")]
fn rebound_and_pressed(world: &mut IntegrationWorld) -> Result<()> {
    let Target::Single(code) = target(SECOND_HOTKEY) else {
        bail!("the second hotkey is not a single key");
    };
    let run = rebind(world)?;
    run.binding.rebind(SECOND_HOTKEY);
    run.signals.clear();
    run.key(KeyEvent::Down(code));
    run.key(KeyEvent::Up(code));
    // the first hotkey, pressed after the change
    run.key(KeyEvent::Down(RIGHT_OPTION));
    Ok(())
}

#[when("the binding changes to a single key and the chord's key is pressed")]
fn rebound_chord_key(world: &mut IntegrationWorld) -> Result<()> {
    let run = rebind(world)?;
    run.binding.rebind(FIRST_HOTKEY);
    run.signals.clear();
    run.key(KeyEvent::Down(KEY_E));
    Ok(())
}

#[then("a press and a release are signalled")]
fn press_and_release(world: &mut IntegrationWorld) -> Result<()> {
    let signals = &rebind(world)?.signals;
    ensure!(
        signals.get(..2) == Some(&[Some(PttSignal::Pressed), Some(PttSignal::Released)][..]),
        "{signals:?}"
    );
    Ok(())
}

#[then("a press of the first hotkey after the change signals nothing")]
fn first_hotkey_signals_nothing(world: &mut IntegrationWorld) -> Result<()> {
    let signals = &rebind(world)?.signals;
    ensure!(signals.get(2) == Some(&None), "{signals:?}");
    Ok(())
}

#[then("nothing is signalled")]
fn nothing_signalled(world: &mut IntegrationWorld) -> Result<()> {
    let signals = &rebind(world)?.signals;
    ensure!(*signals == [None], "{signals:?}");
    Ok(())
}
