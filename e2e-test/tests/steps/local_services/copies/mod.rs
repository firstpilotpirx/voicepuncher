//! Steps of `local_services.feature` about the speech recognition service run as several
//! copies found from its address: clients find the copies, the copies share the requests,
//! one that dies comes back, an abandoned request holds no copy, and stopping leaves nothing
//! behind.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher_copy_pool::{CopyPool, Finish, Ran, ServiceAddress};
use voicepuncher_e2e_test::speech::{synthesize, Condition, Lang};

use super::{
    alive, answers, any_process, pgrep, services, services_dir, wait_until, ScenarioServices,
    SPEECH_RECOGNITION,
};
use crate::steps::off_thread;
use crate::E2eWorld;

mod shared;

use shared::{ensure_all_answered, unanswered_at_once, BusyProcessor};
pub(super) use shared::{short_speech, transcribe};

mod restart;
mod start_failure;
use start_failure::TakenPort;
mod discovery;
mod queueing;
use queueing::answered;
mod giving_up;
use giving_up::{requests_taken, requests_taken_by_copy};

/// "Several copies" in the scenarios: the fewest that can share requests.
const SEVERAL_COPIES: u32 = 2;
/// "Three copies" in the scenarios.
const THREE_COPIES: u32 = 3;
const COPY_BACK_WITHIN: Duration = Duration::from_secs(30);
const PROCESS_GONE_WITHIN: Duration = Duration::from_secs(10);
/// The requests that arrive "one after another" while every copy is busy.
const MORE_REQUESTS: usize = 2;
/// Requests sent at once right after a copy died: more than the copies that still answer.
const REQUESTS_WHILE_DOWN: usize = 4;
/// Between two requests sent "one after another", so they reach the queue in that order.
const BETWEEN_REQUESTS: Duration = Duration::from_millis(200);
/// Silence added to each request sent after the first, so the copies' logs tell them apart
/// by the length of their audio: a tenth of a second.
const PADDING_SAMPLES: usize = 1_600;

/// What the copy scenarios sent and saw: the long requests that keep the copies busy and
/// the requests sent after them, each ending with the moment it was answered.
#[derive(Debug, Default)]
pub struct PoolRun {
    long: Vec<JoinHandle<Result<Instant>>>,
    more: Vec<JoinHandle<Result<Instant>>>,
    /// When each of the `more` requests reached a copy, with the length of its audio, from
    /// the copies' logs, in the order seen.
    reached: Option<JoinHandle<Vec<(Instant, usize)>>>,
    unanswered: Option<Vec<String>>,
    /// The processes keeping every core busy while the requests are shared.
    busy: Option<BusyProcessor>,
    copy_killed: Option<Instant>,
    taken: Option<TakenPort>,
    /// The copies the client found from the service address.
    found: Option<Vec<u16>>,
    /// The copy still transcribing a request its client gave up on.
    abandoned: Option<u16>,
    /// When the next request was answered: the requests the copy of the abandoned one and
    /// the other copy had taken.
    after_abandoned: Option<(usize, usize)>,
}

impl ScenarioServices {
    /// The ports of the copies the service scripts started, from the list they keep.
    pub(super) fn running_copy_ports(&self) -> Vec<u16> {
        self.copy_ports_of(self.stt_port)
    }

    /// The ports of the copies of the speech recognition service on `port`, from the list the
    /// service scripts keep in the scenario's home.
    pub(super) fn copy_ports_of(&self, port: u16) -> Vec<u16> {
        std::fs::read_to_string(
            self.dir("run")
                .join(format!("{SPEECH_RECOGNITION}-{port}.copies")),
        )
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|port| port.parse().ok())
        .collect()
    }
}

fn copy_list(stt_port: u16) -> std::path::PathBuf {
    services_dir("run").join(format!("{SPEECH_RECOGNITION}-{stt_port}.copies"))
}

/// The list of copies the service scripts keep for the service on `stt_port`.
pub(super) fn remove_copy_list(stt_port: u16) {
    let _ = std::fs::remove_file(copy_list(stt_port));
}

/// The copies the client pool of this process finds from the service address `url`.
fn found_copies(url: &str) -> Result<Vec<u16>> {
    let pool = CopyPool::shared(url).map_err(|e| anyhow!("{e}"))?;
    Ok(pool.snapshot().iter().map(|copy| copy.port).collect())
}

fn long_speech() -> Result<Vec<i16>> {
    synthesize(Lang::En, Condition::Normal, 0.0)
}

pub(super) fn started_with(world: &mut E2eWorld, copies: Option<u32>) -> Result<()> {
    let mut services = ScenarioServices::with_copies(copies)?;
    services.start_speech_recognition()?;
    let started = services.run.started.as_ref().expect("just started it");
    ensure!(
        started.status.success(),
        "the service scripts could not start the speech recognition service:\n{}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );
    world.services.local_services = Some(services);
    Ok(())
}

/// The ports the service scripts give the copies of the service on `stt_port`, as a client
/// finds them from its address.
fn expected_copy_ports(stt_port: u16, copies: u32) -> Vec<u32> {
    let address = ServiceAddress::parse(&format!("http://127.0.0.1:{stt_port}/v1"))
        .expect("a loopback address");
    let ports = address.copy_ports().into_iter().map(u32::from);
    ports.take(copies as usize).collect()
}

/// How long the client of the abandoned request waits before it gives up on it.
const GIVES_UP_AFTER: Duration = Duration::from_millis(300);
/// The long requests that keep both copies busy.
const BUSY_REQUESTS: usize = 2;
/// The long speech of a busy copy, this many times the long speech (about a minute of
/// audio each time): still busy when the requests after it arrive (0.2–0.4 s later), yet
/// answered before the waiting requests give up: three waits of 5 s plus half their audio,
/// about 16 s in all, whatever the load of the other checkouts' checks on the GPU. At two
/// and three times, the full-checks of 2026-10-22 saw both copies stay busy past that wait.
const BUSY_SPEECH_REPEATS: usize = 1;
/// The abandoned request, this many times the long speech: still transcribed when the
/// client's next request is answered.
const ABANDONED_SPEECH_REPEATS: usize = 10;
/// How long the long requests may take to reach both copies.
const BUSY_WITHIN: Duration = Duration::from_secs(20);
/// How long the waiting requests may take to reach a copy: the long ones take seconds each.
const REACHED_WITHIN: Duration = Duration::from_secs(120);
/// How often the copies' logs are read for a request they took.
const LOG_POLL: Duration = Duration::from_millis(20);
/// What a copy logs for every request it takes, with the length of its audio.
pub(super) const REQUEST_LOG_LINE: &str = "processing '";
/// The file name of the warm-up the service scripts send each copy before it counts ready.
pub(super) const WARM_UP_FILE: &str = "warm-up.wav'";

/// The requests a copy's `log` shows from its clients, the warm-up left out: each one's line.
pub(super) fn client_requests(log: &str) -> impl Iterator<Item = &str> {
    log.split(REQUEST_LOG_LINE)
        .skip(1)
        .filter(|line| !line.starts_with(WARM_UP_FILE))
}
