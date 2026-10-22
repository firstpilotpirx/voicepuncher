//! The chat requests the app sends to the text refinement service stub.

use anyhow::{bail, Context, Result};
use cucumber::{then, when};
use voicepuncher_e2e_test::refinement_service_stub::{missing_request_report, StubEvent};

use super::{
    expect_success, run_app, speech_recording, Refinement, CORRECTED_TEXT, CUT_OFF_TEXT,
    OUTPUT_LIMIT_LOG, SPEECH, SPEECH_RU, SPEECH_RU_WORDS, SPEECH_WORDS,
};
use crate::steps::off_thread;
use crate::E2eWorld;
use voicepuncher_e2e_test::speech::Lang;

/// Opens the line of one run of a series in the pasted output: `run <n>: <pasted text>`.
const RUN_PREFIX: &str = "run ";
const RUN_SEPARATOR: &str = ": ";

/// The one chat request of the run; otherwise a failure naming every connection the stub
/// saw and the app's log, so a lost transcription is told apart from a missing request.
pub fn only_chat_request(world: &E2eWorld) -> Result<serde_json::Value> {
    let stub = world
        .refinement_stub
        .as_ref()
        .context("no text refinement service stub")?;
    let events = stub.events();
    let requests = stub.chat_requests();
    let [request] = requests.as_slice() else {
        bail!("{}", missing_request_report(&events, &world.stderr));
    };
    serde_json::from_str(request).context("the request to the service is not JSON")
}

pub fn expect_message_role<'a>(message: &'a serde_json::Value, role: &str) -> Result<&'a str> {
    if message["role"] != role {
        bail!("expected a '{role}' message, got {message}");
    }
    message["content"]
        .as_str()
        .with_context(|| format!("the '{role}' message has no text"))
}

/// Runs the app `runs` times on one recording against the scenario's stub; the pasted text
/// of every run stays as one line of the output, the logs one after another.
#[when(regex = r"^the user transcribes the same recording (\d+) times in a row with refinement$")]
async fn transcribe_series(world: &mut E2eWorld, runs: usize) -> Result<()> {
    off_thread(world, move |world| {
        let recording = speech_recording(world, SPEECH)?;
        let mut pasted = Vec::with_capacity(runs);
        let mut logs = Vec::with_capacity(runs);
        for run in 1..=runs {
            run_app(world, &recording, Refinement::On)?;
            let status = world.exit_status.context("the app did not run")?;
            let output = if status.success() {
                world.stdout.clone()
            } else {
                format!("the app failed with {status}")
            };
            pasted.push(format!("{RUN_PREFIX}{run}{RUN_SEPARATOR}{output}"));
            logs.push(format!("--- {RUN_PREFIX}{run} ---\n{}", world.stderr));
        }
        world.stdout = pasted.join("\n");
        world.stderr = logs.join("\n");
        Ok(())
    })
    .await
}

#[then("every run sends the service exactly one request and pastes the raw transcript instead of the cut-off text")]
fn every_run_refined_once(world: &mut E2eWorld) -> Result<()> {
    let runs = world.stdout.lines().count();
    every_run_pasted_raw(world)?;
    one_request_per_run(world, runs)?;
    let limit_logs = world.stderr.matches(OUTPUT_LIMIT_LOG).count();
    if limit_logs < runs {
        bail!(
            "expected '{OUTPUT_LIMIT_LOG}' logged in each of {runs} runs, found it {limit_logs} times; stderr: {}",
            world.stderr
        );
    }
    Ok(())
}

/// Every run line of a series holds the raw transcript.
fn every_run_pasted_raw(world: &E2eWorld) -> Result<()> {
    for line in world.stdout.lines() {
        let (run, output) = line
            .split_once(RUN_SEPARATOR)
            .with_context(|| format!("not a run line: {line:?}"))?;
        raw_transcript_pasted(output, world.spoken)
            .with_context(|| format!("{run}: stderr: {}", world.stderr))?;
    }
    Ok(())
}

/// The stub saw one chat request per run and dropped no connection; the app's probes of
/// the service (connections closed without a request) are expected and not counted.
fn one_request_per_run(world: &E2eWorld, runs: usize) -> Result<()> {
    let stub = world
        .refinement_stub
        .as_ref()
        .context("no text refinement service stub")?;
    let events = stub.events();
    let chat_requests = events
        .iter()
        .filter(|event| matches!(event, StubEvent::Chat { .. }))
        .count();
    let dropped = events
        .iter()
        .any(|event| matches!(event, StubEvent::Dropped { .. }));
    if chat_requests != runs || dropped {
        bail!(
            "expected one request per run over {runs} runs; {}",
            missing_request_report(&events, &world.stderr)
        );
    }
    Ok(())
}

/// The app succeeded and pasted the raw transcript of the spoken sentence.
pub fn expect_raw_transcript_pasted(world: &E2eWorld) -> Result<()> {
    expect_success(world)?;
    raw_transcript_pasted(&world.stdout, world.spoken)
}

/// `pasted` is the raw transcript of the sentence spoken in `lang`: neither the cut-off text
/// nor a correction.
fn raw_transcript_pasted(pasted: &str, lang: Option<Lang>) -> Result<()> {
    let (spoken, spoken_words) = match lang {
        Some(Lang::Ru) => (SPEECH_RU, SPEECH_RU_WORDS),
        _ => (SPEECH, SPEECH_WORDS),
    };
    if !pasted.to_lowercase().contains(spoken_words) || pasted == CORRECTED_TEXT {
        bail!("expected the raw transcript of {spoken:?} to be pasted, got {pasted:?}");
    }
    if pasted == CUT_OFF_TEXT {
        bail!("the cut-off text was pasted");
    }
    Ok(())
}
