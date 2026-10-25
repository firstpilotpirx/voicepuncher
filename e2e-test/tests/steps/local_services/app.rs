//! The app in hotkey mode starting the services it needs itself, on ports of the scenario's
//! own, and stopping them when it is told to end.

use std::process::ExitStatus;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use voicepuncher::adapters::primary::menubar::macos::system_quit::SYSTEM_QUIT_NOTE;
use voicepuncher::adapters::secondary::logging::file::{logs_dir, LOG_FILE_NAME};

use super::{answers, healthy, service_process, services, signal, wait_until, ScenarioServices};
use crate::steps::file_transcription::sut_bin;
use crate::steps::off_thread;
use crate::E2eWorld;

/// The models the app's speech recognition service needs, as the `justfile` exports them.
const MODELS: [&str; 2] = ["VOICEPUNCHER_WHISPER_MODEL", "VOICEPUNCHER_VAD_MODEL"];
/// Loading the dictation model takes 5–20 s; the app gives a start 60 s.
const READY_WITHIN: Duration = Duration::from_secs(90);
/// The app asks its services to end and kills one after 5 s.
const ENDED_WITHIN: Duration = Duration::from_secs(15);
/// What the app logs when a service it started answers.
const READY_NOTE: &str = "ready in";

/// The app under test and what the scenario saw while it ran.
#[derive(Debug, Default)]
pub struct LaunchedApp {
    home: Option<tempfile::TempDir>,
    child: Option<Child>,
    answered: bool,
    exit: Option<ExitStatus>,
}

impl Drop for LaunchedApp {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl LaunchedApp {
    fn log(&self) -> String {
        self.home
            .as_ref()
            .map(|home| logs_dir(home.path()).join(LOG_FILE_NAME))
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default()
    }
}

fn app(world: &mut E2eWorld) -> Result<&mut LaunchedApp> {
    services(world)?
        .app
        .as_mut()
        .context("the app was not launched")
}

#[given("no local service runs on the chosen ports and the models are on disk")]
fn ports_free_models_on_disk(world: &mut E2eWorld) -> Result<()> {
    for variable in MODELS {
        let path = std::env::var(variable).with_context(|| format!("{variable} is not set"))?;
        ensure!(
            std::path::Path::new(&path).is_file(),
            "{variable} names no file: {path} — run 'just models'"
        );
    }
    world.services.local_services = Some(ScenarioServices::on_free_ports()?);
    Ok(())
}

#[when("the app launches, its services become ready and the app is told to end")]
async fn app_launches_and_ends(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        launch_ready_and_end(world, |pid| signal(pid, "-TERM"))
    })
    .await
}

#[when("the app launches, its services become ready and the system tells the app to quit")]
async fn app_launches_and_system_quits(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        launch_ready_and_end(world, quit_from_the_system)
    })
    .await
}

/// Launches the app on the scenario's port, waits for its service, then ends it with `end`.
fn launch_ready_and_end(world: &mut E2eWorld, end: fn(u32) -> Result<()>) -> Result<()> {
    let port = services(world)?.stt_port;
    let home = tempfile::tempdir()?;
    let child = spawn_app(home.path(), port)?;
    let pid = child.id();
    let launched = LaunchedApp { home: Some(home), child: Some(child), ..LaunchedApp::default() };
    services(world)?.app = Some(launched);
    let app = app(world)?;
    let ready = wait_until(READY_WITHIN, || app.log().contains(READY_NOTE));
    app.answered = ready && healthy(port);
    end_app(app, pid, end)
}

/// Ends the app with `end` and keeps how it exited.
fn end_app(app: &mut LaunchedApp, pid: u32, end: fn(u32) -> Result<()>) -> Result<()> {
    end(pid)?;
    let child = app.child.as_mut().context("the app is gone")?;
    app.exit = wait_for_exit(child, ENDED_WITHIN)?;
    Ok(())
}

/// Starts the app in `home`, a home of its own: the app logs, and its services log, there,
/// never to the user's; it manages one copy of the dictation service on `port`.
fn spawn_app(home: &std::path::Path, port: u16) -> Result<std::process::Child> {
    let config = home.join("config.toml");
    std::fs::write(
        &config,
        format!(
            "[transcription]\nbase_url = \"http://127.0.0.1:{port}/v1\"\n\n\
             [services]\nmanage = true\ndictation_copies = 1\n"
        ),
    )?;
    Command::new(sut_bin())
        .arg("--config")
        .arg(&config)
        .env("HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn the app")
}

/// Sends the app the quit Apple Event that logout, shutdown and Activity Monitor send.
fn quit_from_the_system(pid: u32) -> Result<()> {
    let script = format!(
        "ObjC.import('AppKit'); \
         const app = $.NSRunningApplication.runningApplicationWithProcessIdentifier({pid}); \
         app.isNil() ? 'not running' : (app.terminate ? 'sent' : 'refused')"
    );
    let output = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .context("failed to run osascript")?;
    let answer = String::from_utf8_lossy(&output.stdout);
    ensure!(
        answer.trim() == "sent",
        "the quit was not sent to the app {pid}: {} {}",
        answer.trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

fn wait_for_exit(child: &mut Child, limit: Duration) -> Result<Option<ExitStatus>> {
    let deadline = std::time::Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if std::time::Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[then("each started service answered on its port while the app ran")]
fn started_service_answered(world: &mut E2eWorld) -> Result<()> {
    let app = app(world)?;
    if !app.answered {
        bail!(
            "the speech recognition service never answered; app log:\n{}",
            app.log()
        );
    }
    Ok(())
}

#[then("no process of them is left after the app ended")]
fn no_process_left(world: &mut E2eWorld) -> Result<()> {
    let port = services(world)?.stt_port;
    let app = app(world)?;
    match app.exit {
        Some(status) if status.success() => {}
        Some(status) => bail!("the app ended with {status}; app log:\n{}", app.log()),
        None => bail!("the app did not end within {ENDED_WITHIN:?} of the termination signal"),
    }
    if answers(port) || service_process(port).is_some() {
        bail!("the speech recognition service on port {port} is left after the app ended");
    }
    Ok(())
}

#[then("the app's log names the quit from the system")]
fn log_names_system_quit(world: &mut E2eWorld) -> Result<()> {
    let app = app(world)?;
    let log = app.log();
    ensure!(
        log.contains(SYSTEM_QUIT_NOTE),
        "the app logged no quit from the system:\n{log}"
    );
    Ok(())
}
