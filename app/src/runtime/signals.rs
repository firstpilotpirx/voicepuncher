//! The hang-up signal reloads the configuration file the runtime was started with; a
//! termination or interrupt signal stops the local services the app started and ends it.

use std::path::PathBuf;
use std::thread::JoinHandle;

use anyhow::{Context, Result};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::config::{self, Config};

use super::{log_rejection, Reloader};

/// What the command line sets over the file, applied again on every reload.
pub type Overrides = Box<dyn Fn(&mut Config) + Send>;

/// Reloads from `path`, with `overrides` applied, on every hang-up signal the process gets
/// from now on; a file that does not load is rejected as any rejected configuration.
pub fn reload_on_hang_up(
    path: PathBuf,
    overrides: Overrides,
    reloader: Reloader,
) -> Result<JoinHandle<()>> {
    // registered before the thread starts: a signal arriving at once is already handled
    let mut signals = Signals::new([SIGHUP]).context("failed to handle the hang-up signal")?;
    std::thread::Builder::new()
        .name("signals".into())
        .spawn(move || {
            for _ in signals.forever() {
                log::info!(
                    "runtime: reloading configuration from {} (hang-up signal)",
                    path.display()
                );
                match config::load(&path) {
                    Ok(mut config) => {
                        overrides(&mut config);
                        // a rejection is logged by the reloader
                        let _ = reloader.reload(config);
                    }
                    Err(e) => log_rejection(&e),
                }
            }
        })
        .context("failed to spawn signals thread")
}

/// The exit code after a termination or interrupt signal.
pub const TERMINATED_EXIT_CODE: i32 = 0;

/// On the first termination or interrupt signal, runs `stop` and ends the process.
pub fn stop_on_termination(stop: Box<dyn FnOnce() + Send>) -> Result<JoinHandle<()>> {
    let mut signals =
        Signals::new([SIGTERM, SIGINT]).context("failed to handle the termination signals")?;
    std::thread::Builder::new()
        .name("termination".into())
        .spawn(move || {
            if let Some(signal) = signals.forever().next() {
                log::info!("runtime: signal {signal}: stopping the services the app started");
                stop();
                std::process::exit(TERMINATED_EXIT_CODE);
            }
        })
        .context("failed to spawn the termination thread")
}
