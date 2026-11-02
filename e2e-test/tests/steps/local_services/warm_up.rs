//! Steps of `local_services.feature` about the warm-up of a speech recognition copy: the
//! service scripts report a copy ready only after its first transcription.

use cucumber::{then, when};
use regex::Regex;
use voicepuncher::test_doubles::prelude::*;

use super::copies::{REQUEST_LOG_LINE, WARM_UP_FILE};
use super::{services, ScenarioServices, SPEECH_RECOGNITION};
use crate::steps::off_thread;
use crate::E2eWorld;

/// How the service scripts name the service for dictation in their output.
const DICTATION_LABEL: &str = "speech recognition service for dictation";

impl ScenarioServices {
    /// The ready line the service scripts printed for each copy of the service for dictation,
    /// by the copy's port: the time of its first transcription, if the line names one.
    fn dictation_copies_ready(&self) -> Result<Vec<(u16, Option<u64>)>> {
        let (_, output) = self
            .run
            .starts
            .iter()
            .find(|(name, _)| name == SPEECH_RECOGNITION)
            .context("the speech recognition service for dictation was never started")?;
        let printed = String::from_utf8_lossy(&output.stdout);
        let ready = Regex::new(&format!(
            r"{DICTATION_LABEL}: copy \d+ of \d+ ready on port (\d+)(?:, first transcription (\d+) ms)?"
        ))?;
        ready
            .captures_iter(&printed)
            .map(|line| {
                let port = line[1].parse()?;
                let took = line.get(2).map(|ms| ms.as_str().parse()).transpose()?;
                Ok((port, took))
            })
            .collect()
    }
}

#[when("the service scripts start the speech recognition service for dictation")]
async fn start_dictation_service(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        // WHY: a scenario starts one copy by default, and one copy is started without warm-up.
        services.options.copies = Some(2);
        services.start_each(&[SPEECH_RECOGNITION])
    })
    .await
}

#[then("each copy is reported ready with the time of its first transcription, and the copy's log shows that transcription before the report")]
async fn copies_ready_after_transcription(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let ports = services.copy_ports_of(services.stt_port);
        ensure!(
            !ports.is_empty(),
            "the service scripts started no copies for dictation"
        );
        let ready = services.dictation_copies_ready()?;
        ports
            .iter()
            .try_for_each(|port| services.check_warmed(*port, &ready))
    })
    .await
}

impl ScenarioServices {
    /// The copy on `port` was reported ready with its first transcription's time, and its log
    /// shows that transcription.
    fn check_warmed(&self, port: u16, ready: &[(u16, Option<u64>)]) -> Result<()> {
        match ready.iter().find(|(p, _)| *p == port) {
            Some((_, Some(_))) => {}
            Some((_, None)) => {
                bail!("copy {port} was reported ready with no first transcription: {ready:?}")
            }
            None => bail!("copy {port} was never reported ready: {ready:?}"),
        }
        let log = self
            .dir("logs")
            .join(format!("{SPEECH_RECOGNITION}-{port}.log"));
        let text = std::fs::read_to_string(&log)
            .with_context(|| format!("no log of copy {port} at {}", log.display()))?;
        ensure!(
            text.contains(&format!("{REQUEST_LOG_LINE}{WARM_UP_FILE}")),
            "the log of copy {port} shows no warm-up transcription: {}",
            log.display()
        );
        Ok(())
    }
}
