//! What the speech recognition client sends and how a failure is judged and reported: the
//! same request for the same audio, an invalid argument taken as a dropped connection, and a
//! copy that sent no answer reported with the machine's load.

use std::io;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::http::failure::is_dropped_io;
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher_e2e_test::machine_load;
use voicepuncher_e2e_test::unanswered_copies::load_beside;

use super::{service_run, ServiceRun};
use crate::IntegrationWorld;

/// The system error macOS gives for a connection the service already reset.
const INVALID_ARGUMENT: i32 = 22;
/// The peak of the speech and of the noise alike: the noise is as loud as the speech.
const PEAK: i32 = 8_192;

#[given("a recording of speech under high noise")]
fn noisy_recording(world: &mut IntegrationWorld) -> Result<()> {
    let mut state = 0x2545_f491_u32;
    let samples = (0..AudioBuffer16k::SAMPLE_RATE_HZ)
        .map(|i| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = i32::try_from(state >> 18).unwrap_or(0) - PEAK;
            let voice = i32::try_from(i % 64).unwrap_or(0) * PEAK / 32 - PEAK;
            i16::try_from(i32::midpoint(voice, noise)).unwrap_or(0)
        })
        .collect();
    service_run(world)?.audio = Some(AudioBuffer16k::new(samples));
    Ok(())
}

/// The value of the multipart field `name` in a request body.
fn field<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("name=\"{name}\"\r\n\r\n");
    let start = body.find(&marker)? + marker.len();
    body[start..].split("\r\n").next()
}

#[then("both requests are the same and ask for no sampling randomness")]
fn same_requests(world: &mut IntegrationWorld) -> Result<()> {
    let run = service_run(world)?;
    let taken = run.service.as_ref().context("no service")?.taken();
    let [first, second] = taken.as_slice() else {
        bail!("{} requests instead of 2", taken.len());
    };
    if first.body != second.body {
        bail!("the two requests for the same recording differ");
    }
    let body = String::from_utf8_lossy(&first.body);
    let sampling = (field(&body, "temperature"), field(&body, "temperature_inc"));
    if sampling != (Some("0"), Some("0")) {
        bail!("the request samples with temperature and its step {sampling:?}");
    }
    Ok(())
}

#[then("both transcripts are the same text")]
fn same_transcripts(world: &mut IntegrationWorld) -> Result<()> {
    let outcomes = &service_run(world)?.outcomes;
    match outcomes.as_slice() {
        [Ok(Some(first)), Ok(Some(second))] if first == second => Ok(()),
        _ => bail!("the two transcriptions gave {outcomes:?}"),
    }
}

#[given("the operating system reports a dropped connection as an invalid argument")]
fn invalid_argument(world: &mut IntegrationWorld) {
    world.extra.speech.service = Some(ServiceRun::default());
}

#[when("the app judges that failure")]
fn judge(world: &mut IntegrationWorld) -> Result<()> {
    let error = io::Error::from_raw_os_error(INVALID_ARGUMENT);
    let verdict = if is_dropped_io(&error) {
        "dropped"
    } else {
        "other"
    };
    service_run(world)?.verdict = Some(format!("{error}: {verdict}"));
    Ok(())
}

#[then("it is taken as a dropped connection, whose recording is sent again")]
fn taken_as_dropped(world: &mut IntegrationWorld) -> Result<()> {
    let verdict = service_run(world)?
        .verdict
        .clone()
        .context("nothing was judged")?;
    if !verdict.ends_with(": dropped") {
        bail!("the failure was judged as {verdict}");
    }
    Ok(())
}

#[when("the check reports what the copies of the service did")]
fn report(world: &mut IntegrationWorld) -> Result<()> {
    let run = service_run(world)?;
    let log = run.warnings.join("\n");
    run.verdict = load_beside(&log, machine_load::report);
    Ok(())
}

#[then(
    "the report names the copy that did not answer, the busiest processes on the machine and every check that held a lock"
)]
fn report_names_load(world: &mut IntegrationWorld) -> Result<()> {
    let run = service_run(world)?;
    let report = run
        .verdict
        .clone()
        .context("the check reported no copy that did not answer")?;
    let hung = run.service.as_ref().context("no service")?.taken()[0].port;
    let names_locks = report.contains("lock holders:")
        || report.contains("no run holds a machine-wide test lock");
    if !report.starts_with(&format!("copies {hung} "))
        || !report.contains("busiest processes")
        || !names_locks
    {
        bail!("the report misses the copy {hung}, the processes or the locks: {report}");
    }
    Ok(())
}
