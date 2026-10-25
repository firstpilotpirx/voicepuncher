//! A first launch: the app's own command line started over a configuration file in the
//! settings scenario's folder, and the built-in default file read as the app reads it.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{then, when};
use toml::Value;

use voicepuncher::adapters::primary::settings::macos::{field::Field, table::FIELDS};
use voicepuncher::config::{parse, DEFAULT_FILE};

use crate::steps::settings_window::settings;
use crate::IntegrationWorld;

/// What a first-launch scenario shares between its steps.
#[derive(Debug, Default)]
pub struct FirstLaunchRun {
    output: Option<Output>,
    /// The default file's configuration and an empty file's.
    read: Option<(Value, Value)>,
    /// The options the window shows that the default file leaves without a comment above.
    uncommented: Option<Vec<String>>,
}

fn launch(world: &mut IntegrationWorld) -> &mut FirstLaunchRun {
    world
        .extra
        .ui
        .first_launch
        .get_or_insert_with(FirstLaunchRun::default)
}

/// An address on this machine where nothing listens.
fn refused_url() -> Result<String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(format!(
        "http://127.0.0.1:{}/v1",
        listener.local_addr()?.port()
    ))
}

/// The configuration of `text`, as a value to compare.
fn configuration(text: &str, origin: &str) -> Result<Value> {
    Ok(Value::try_from(parse(text, origin)?)?)
}

/// The options the default file sets with a comment line right above them, as `section.key`.
fn commented_options() -> Vec<String> {
    let (mut section, mut above) = (String::new(), String::new());
    let mut options = Vec::new();
    for line in DEFAULT_FILE.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            name.clone_into(&mut section);
        } else if let Some((key, _)) = line.split_once('=').filter(|_| above.starts_with('#')) {
            options.push(format!("{section}.{}", key.trim()));
        }
        line.clone_into(&mut above);
    }
    options
}

/// Starts the app over the scenario's configuration file; a recording that is no WAV file
/// ends the run once the configuration was read.
#[when("the app starts over that configuration file")]
fn app_starts(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let recording = run.folder.path().join("not-a-recording.wav");
    std::fs::write(&recording, "not a recording")?;
    let output = Command::new(env!("CARGO_BIN_EXE_voicepuncher"))
        .arg("--config")
        .arg(run.path())
        .arg("--input-file")
        .arg(&recording)
        .args(["--transcription-url", &refused_url()?])
        .env("HOME", run.folder.path())
        .stdin(Stdio::null())
        .output()?;
    launch(world).output = Some(output);
    Ok(())
}

#[when("the default file is read as a configuration")]
fn default_file_read(world: &mut IntegrationWorld) -> Result<()> {
    let default = configuration(DEFAULT_FILE, "the default file")?;
    let empty = configuration("", "an empty file")?;
    launch(world).read = Some((default, empty));
    Ok(())
}

#[when("the options of the default file are compared with the options the configuration accepts")]
fn default_options_compared(world: &mut IntegrationWorld) {
    let commented = commented_options();
    let uncommented = FIELDS
        .iter()
        .filter(|field| !commented.contains(&field.path()))
        .map(Field::path)
        .collect();
    launch(world).uncommented = Some(uncommented);
}

#[then("the configuration file exists with the default file's text")]
fn file_holds_defaults(world: &mut IntegrationWorld) -> Result<()> {
    let written = settings(world).file()?;
    if written != DEFAULT_FILE {
        bail!("the configuration file holds:\n{written}");
    }
    Ok(())
}

#[then("the log says the file was created with the defaults")]
fn log_says_created(world: &mut IntegrationWorld) -> Result<()> {
    let path = settings(world).path();
    let output = launch(world)
        .output
        .as_ref()
        .context("the app did not start")?;
    let printed = String::from_utf8_lossy(&output.stderr);
    let wanted = format!("config: created {} with the defaults", path.display());
    if !printed.contains(&wanted) {
        bail!("the log does not say {wanted:?}:\n{printed}");
    }
    Ok(())
}

#[then("it equals the configuration of an empty file")]
fn equals_empty_file(world: &mut IntegrationWorld) -> Result<()> {
    let (default, empty) = launch(world).read.clone().context("nothing was read")?;
    if default != empty {
        bail!("the default file reads as\n{default:#?}\nan empty file as\n{empty:#?}");
    }
    Ok(())
}

#[then(
    "every option the settings window shows is set in the default file, with a comment above it"
)]
fn every_option_commented(world: &mut IntegrationWorld) -> Result<()> {
    let uncommented = launch(world)
        .uncommented
        .clone()
        .context("nothing was compared")?;
    if !uncommented.is_empty() {
        bail!("not set with a comment above: {uncommented:?}");
    }
    Ok(())
}
