//! Steps of `local_services.feature` about the speech recognition service run as several
//! copies found from its address: clients find the copies, the copies share the requests,
//! one that dies comes back, an abandoned request holds no copy, and stopping leaves nothing
//! behind.

use std::process::Command;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{anyhow, ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::Transcriber;
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher_copy_pool::{CopyPool, Finish, Ran, ServiceAddress};
use voicepuncher_e2e_test::speech::{synthesize, Condition, Lang};
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use super::{
    alive, answers, any_process, pgrep, services, services_dir, wait_until, ScenarioServices,
    SPEECH_RECOGNITION,
};
use crate::steps::off_thread;
use crate::E2eWorld;

/// How many copies the service scripts start when no number is set.
const DEFAULT_COPIES: usize = 3;
/// "Several copies" in the scenarios: the fewest that can share requests.
const SEVERAL_COPIES: u32 = 2;
const REQUESTS_AT_ONCE: usize = 9;
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
/// What a copy logs for every request it transcribes.
const TRANSCRIBED_LOG_LINE: &str = "whisper_vad: detected";
const SHORT_UTTERANCE: &str = "press enter";

/// What the copy scenarios sent and saw: the long requests that keep the copies busy and
/// the requests sent after them, each ending with the moment it was answered.
#[derive(Debug, Default)]
pub struct PoolRun {
    long: Vec<JoinHandle<Result<Instant>>>,
    more: Vec<JoinHandle<Result<Instant>>>,
    /// When each of the `more` requests reached a copy, from the copies' logs.
    reached: Option<JoinHandle<Vec<Instant>>>,
    unanswered: Option<Vec<String>>,
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

/// The transcript the service at `url` gives for `samples`, through the app's own client.
pub(super) fn transcribe(url: &str, samples: Vec<i16>) -> Result<String> {
    let mut client = OpenAiTranscriber::connect(TranscriptionCfg {
        base_url: url.to_owned(),
        ..TranscriptionCfg::default()
    })?;
    let transcript = client.transcribe(&AudioBuffer16k::new(samples))?;
    Ok(transcript
        .map(|t| t.as_str().to_owned())
        .unwrap_or_default())
}

/// Each of `count` short requests sent at once to `url` that got no transcript, numbered from 1,
/// with its error chain or "empty transcript".
fn unanswered_at_once(url: &str, count: usize) -> Result<Vec<String>> {
    let speech = short_speech()?;
    let requests: Vec<_> = (0..count)
        .map(|_| {
            let (url, speech) = (url.to_owned(), speech.clone());
            std::thread::spawn(move || transcribe(&url, speech))
        })
        .collect();
    Ok(requests
        .into_iter()
        .enumerate()
        .filter_map(|(index, request)| {
            let failure = match request.join() {
                Ok(Ok(text)) if !text.is_empty() => return None,
                Ok(Ok(_)) => "empty transcript".to_owned(),
                Ok(Err(error)) => format!("{error:#}"),
                Err(_) => "its thread panicked".to_owned(),
            };
            Some(format!("request {}: {failure}", index + 1))
        })
        .collect())
}

/// Fails naming every request in `unanswered` when any of `sent` got no transcript.
fn ensure_all_answered(unanswered: &[String], sent: usize, when: &str) -> Result<()> {
    ensure!(
        unanswered.is_empty(),
        "{} of {sent} requests got no transcript{when}:\n{}",
        unanswered.len(),
        unanswered.join("\n")
    );
    Ok(())
}

pub(super) fn short_speech() -> Result<Vec<i16>> {
    synthesized(&SayRequest {
        voice: None,
        words_per_minute: None,
        text: SHORT_UTTERANCE,
    })
}

fn long_speech() -> Result<Vec<i16>> {
    synthesize(Lang::En, Condition::Normal, 0.0)
}

fn started_with(world: &mut E2eWorld, copies: Option<u32>) -> Result<()> {
    let mut services = ScenarioServices::with_copies(copies)?;
    services.start_speech_recognition()?;
    let started = services.started.as_ref().expect("just started it");
    ensure!(
        started.status.success(),
        "the service scripts could not start the speech recognition service:\n{}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );
    world.local_services = Some(services);
    Ok(())
}

#[given(
    "the service scripts started the speech recognition service for dictation without a number of copies"
)]
async fn started_by_default(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, None)).await
}

#[given("the service scripts started the speech recognition service as several copies")]
async fn started_as_copies(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(SEVERAL_COPIES))).await
}

#[when("several transcription requests arrive at once")]
async fn requests_at_once(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        services.pool.unanswered = Some(unanswered_at_once(
            &services.transcription_url(),
            REQUESTS_AT_ONCE,
        )?);
        Ok(())
    })
    .await
}

#[when("one copy exits on its own")]
async fn copy_dies(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = *services
            .copy_ports
            .first()
            .context("the service scripts started no copies")?;
        let pid = pgrep(&format!("^{SPEECH_RECOGNITION} .*--port {port}( |$)"))
            .context("no process of the first copy was found")?;
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
        services.pool.copy_killed = Some(Instant::now());
        wait_until(PROCESS_GONE_WITHIN, || !alive(pid));
        Ok(())
    })
    .await
}

#[then("every request gets its transcript, and each of the three copies transcribed some of them")]
fn shared_by_copies(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let unanswered = services
        .pool
        .unanswered
        .as_deref()
        .context("no requests were sent")?;
    ensure_all_answered(unanswered, REQUESTS_AT_ONCE, "")?;
    ensure!(
        services.copy_ports.len() == DEFAULT_COPIES,
        "{} copies run, not {DEFAULT_COPIES}: {:?}",
        services.copy_ports.len(),
        services.copy_ports
    );
    for port in &services.copy_ports {
        let log = services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{port}.log"));
        let text =
            std::fs::read_to_string(&log).with_context(|| format!("log {}", log.display()))?;
        ensure!(
            text.contains(TRANSCRIBED_LOG_LINE),
            "the copy on port {port} transcribed none of the requests"
        );
    }
    Ok(())
}

#[then("the service keeps answering transcription requests, and that copy answers again within 30 seconds")]
async fn copy_came_back(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = *services.copy_ports.first().context("no copies")?;
        let killed = services.pool.copy_killed.context("no copy was killed")?;
        let down = !answers(port);
        let unanswered = unanswered_at_once(&services.transcription_url(), REQUESTS_WHILE_DOWN)?;
        let when = if down {
            " while a copy was down (it was down when they were sent)"
        } else {
            " while a copy was down (it was already back)"
        };
        ensure_all_answered(&unanswered, REQUESTS_WHILE_DOWN, when)?;
        let back_within = COPY_BACK_WITHIN.saturating_sub(killed.elapsed());
        ensure!(
            wait_until(back_within, || answers(port)),
            "the copy on port {port} did not answer within {COPY_BACK_WITHIN:?} of its exit"
        );
        Ok(())
    })
    .await
}

#[then("no copy of it is left, and nothing answers on any of its ports")]
async fn no_copy_left(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        ensure!(
            !services.copy_ports.is_empty(),
            "the start left no copies to check"
        );
        for port in &services.copy_ports {
            ensure!(
                wait_until(PROCESS_GONE_WITHIN, || any_process(*port).is_none()),
                "the copy on port {port} is left: {:?}",
                any_process(*port)
            );
        }
        let ports = std::iter::once(services.stt_port).chain(services.copy_ports.iter().copied());
        for port in ports {
            ensure!(
                !answers(port),
                "port {port} of the speech recognition service still answers after it was stopped"
            );
        }
        ensure!(
            !copy_list(services.stt_port).exists(),
            "the list of copies is left"
        );
        Ok(())
    })
    .await
}

/// The ports the service scripts give the copies of the service on `stt_port`, as a client
/// finds them from its address.
fn expected_copy_ports(stt_port: u16, copies: u32) -> Vec<u32> {
    let address = ServiceAddress::parse(&format!("http://127.0.0.1:{stt_port}/v1"))
        .expect("a loopback address");
    let ports = address.copy_ports().into_iter().map(u32::from);
    ports.take(copies as usize).collect()
}

/// Holds `port` bound without listening, so a server cannot bind it while nothing answers
/// there; it is released when dropped.
#[derive(Debug)]
pub struct TakenPort(std::process::Child);

impl TakenPort {
    fn hold(port: u32) -> Result<Self> {
        let mut child = Command::new("python3")
            .args([
                "-c",
                "import socket, sys, time\n\
                 s = socket.socket(); s.bind(('127.0.0.1', int(sys.argv[1])))\n\
                 print('bound', flush=True); time.sleep(600)",
                &port.to_string(),
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .context("python3 runs")?;
        let mut line = String::new();
        std::io::BufRead::read_line(
            &mut std::io::BufReader::new(child.stdout.as_mut().context("its output")?),
            &mut line,
        )?;
        ensure!(line.trim() == "bound", "could not take port {port}");
        Ok(Self(child))
    }
}

impl Drop for TakenPort {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[given(
    "the speech recognition service is to run as several copies, and its second copy cannot start"
)]
fn second_copy_cannot_start(world: &mut E2eWorld) -> Result<()> {
    let mut services = ScenarioServices::with_copies(Some(SEVERAL_COPIES))?;
    let second = expected_copy_ports(services.stt_port, SEVERAL_COPIES)[1];
    services.pool.taken = Some(TakenPort::hold(second)?);
    world.local_services = Some(services);
    Ok(())
}

#[given("a list of copies is left from an earlier start, and the service scripts started the speech recognition service as one copy")]
async fn stale_copy_list(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::with_copies(Some(1))?;
        let left = expected_copy_ports(services.stt_port, SEVERAL_COPIES);
        let listed: Vec<String> = left.iter().map(u32::to_string).collect();
        std::fs::write(copy_list(services.stt_port), listed.join(" "))?;
        services.start_speech_recognition()?;
        let started = services.started.as_ref().expect("just started it");
        ensure!(
            started.status.success(),
            "the service scripts could not start the speech recognition service:\n{}",
            String::from_utf8_lossy(&started.stderr)
        );
        world.local_services = Some(services);
        Ok(())
    })
    .await
}

#[then("the start fails naming the service and the log of the copy that failed, and the copy that started is not left")]
async fn copies_start_failed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let started = services.started.as_ref().context("nothing was started")?;
        let report = String::from_utf8_lossy(&started.stderr).into_owned();
        ensure!(
            !started.status.success(),
            "the start succeeded although a copy could not start"
        );
        let stt_port = services.stt_port;
        let ports = expected_copy_ports(stt_port, SEVERAL_COPIES);
        let failed_log =
            services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{}.log", ports[1]));
        ensure!(
            report.contains(SPEECH_RECOGNITION) && report.contains(&*failed_log.to_string_lossy()),
            "the failure report does not name {SPEECH_RECOGNITION} and the log {}:\n{report}",
            failed_log.display()
        );
        let first = u16::try_from(ports[0])?;
        ensure!(
            wait_until(PROCESS_GONE_WITHIN, || any_process(first).is_none()),
            "the copy that started (port {first}) is left: {:?}",
            any_process(first)
        );
        ensure!(
            !copy_list(stt_port).exists(),
            "the list of copies is left after the failed start"
        );
        Ok(())
    })
    .await
}

/// How long the client of the abandoned request waits before it gives up on it.
const GIVES_UP_AFTER: Duration = Duration::from_millis(300);
/// The long requests that keep both copies busy.
const BUSY_REQUESTS: usize = 2;
/// The long speech of a busy copy, this many times the long speech: still busy when the
/// requests after it arrive.
const BUSY_SPEECH_REPEATS: usize = 3;
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

#[given("the service scripts started the speech recognition service for dictation as three copies")]
async fn started_as_three(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(THREE_COPIES))).await
}

#[given("the service scripts started the speech recognition service for dictation as one copy")]
async fn started_as_one(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(1))).await
}

#[given("the speech recognition service for dictation runs as two copies")]
async fn started_as_two(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(SEVERAL_COPIES))).await
}

#[when("a client connects to the service by its address")]
async fn client_connects(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let url = services.transcription_url();
        let text = transcribe(&url, short_speech()?)?;
        ensure!(!text.is_empty(), "the client got an empty transcript");
        services.pool.found = Some(found_copies(&url)?);
        Ok(())
    })
    .await
}

#[then("the client sends its requests to the three copies, and nothing listens on the service's own port")]
fn found_three(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let found = services.pool.found.clone().context("no client connected")?;
    ensure!(
        services.copy_ports.len() == THREE_COPIES as usize && found == services.copy_ports,
        "the client found the copies {found:?}, the service scripts started {:?}",
        services.copy_ports
    );
    ensure!(
        !answers(services.stt_port),
        "something listens on the service's own port {}",
        services.stt_port
    );
    Ok(())
}

#[then("the client sends its requests to that copy on the service's own port")]
fn found_one(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let found = services.pool.found.clone().context("no client connected")?;
    ensure!(
        found == [services.stt_port],
        "the client found the copies {found:?}, not the service's own port {}",
        services.stt_port
    );
    Ok(())
}

#[given(
    "the speech recognition service for dictation runs as two copies, both busy with long speech"
)]
async fn copies_busy(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        started_with(world, Some(SEVERAL_COPIES))?;
        let services = services(world)?;
        let url = services.transcription_url();
        let speech: Vec<i16> = long_speech()?.repeat(BUSY_SPEECH_REPEATS);
        for _ in 0..BUSY_REQUESTS {
            let (url, speech) = (url.clone(), speech.clone());
            services.pool.long.push(std::thread::spawn(move || {
                transcribe(&url, speech)?;
                Ok(Instant::now())
            }));
            std::thread::sleep(BETWEEN_REQUESTS);
        }
        ensure!(
            wait_until(BUSY_WITHIN, || requests_taken(&services.copy_ports)
                >= BUSY_REQUESTS),
            "the copies did not both take a long request within {BUSY_WITHIN:?}"
        );
        Ok(())
    })
    .await
}

#[when("two more requests arrive one after another")]
async fn more_requests(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let url = services.transcription_url();
        let speech = short_speech()?;
        let ports = services.copy_ports.clone();
        services.pool.reached = Some(std::thread::spawn(move || {
            let mut reached = Vec::new();
            let started = Instant::now();
            while reached.len() < MORE_REQUESTS && started.elapsed() < REACHED_WITHIN {
                if requests_taken(&ports) > BUSY_REQUESTS + reached.len() {
                    reached.push(Instant::now());
                    continue;
                }
                std::thread::sleep(LOG_POLL);
            }
            reached
        }));
        for _ in 0..MORE_REQUESTS {
            let (url, speech) = (url.clone(), speech.clone());
            services.pool.more.push(std::thread::spawn(move || {
                let text = transcribe(&url, speech)?;
                ensure!(!text.is_empty(), "a request got an empty transcript");
                Ok(Instant::now())
            }));
            std::thread::sleep(BETWEEN_REQUESTS);
        }
        Ok(())
    })
    .await
}

/// The moments the requests in `requests` were answered, in the order they were sent.
fn answered(requests: Vec<JoinHandle<Result<Instant>>>) -> Result<Vec<Instant>> {
    requests
        .into_iter()
        .map(|request| {
            request
                .join()
                .map_err(|_| anyhow!("a request thread panicked"))?
        })
        .collect()
}

#[then("each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once")]
async fn waited_their_turn(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let long = answered(std::mem::take(&mut services.pool.long))?;
        let more = answered(std::mem::take(&mut services.pool.more))?;
        let first_free = *long.iter().min().context("no long request was sent")?;
        ensure!(
            more.iter().all(|at| *at >= first_free),
            "a request was answered while every copy was still busy"
        );
        let reached = services
            .pool
            .reached
            .take()
            .context("no request was sent")?
            .join()
            .map_err(|_| anyhow!("the log watcher panicked"))?;
        ensure!(
            reached.len() == MORE_REQUESTS,
            "only {} of {MORE_REQUESTS} requests reached a copy",
            reached.len()
        );
        // The n-th waiting request may start only once n copies freed: a long request
        // answered, or an earlier waiting one
        let mut freed: Vec<Instant> = long.iter().chain(&more).copied().collect();
        freed.sort();
        for (n, at) in reached.iter().enumerate() {
            ensure!(
                *at >= freed[n],
                "request {} reached a copy {:?} before a copy was free for it: a copy took two requests at once",
                BUSY_REQUESTS + n + 1,
                freed[n] - *at
            );
        }
        ensure!(
            more.windows(2).all(|pair| pair[0] <= pair[1]),
            "the second request was answered before the first one"
        );
        let taken = requests_taken(&services.copy_ports);
        ensure!(
            taken == BUSY_REQUESTS + MORE_REQUESTS,
            "the copies took {taken} requests, not {}",
            BUSY_REQUESTS + MORE_REQUESTS
        );
        Ok(())
    })
    .await
}

#[given("a client gave up on a request that one copy is still transcribing")]
async fn gave_up_on_a_request(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let pool = CopyPool::shared(&services.transcription_url()).map_err(|e| anyhow!("{e}"))?;
        let lease = pool.lease(BUSY_WITHIN)?;
        let port = lease.port();
        let speech: Vec<i16> = long_speech()?.repeat(ABANDONED_SPEECH_REPEATS);
        let ran = lease.run(GIVES_UP_AFTER, move |copy| {
            (transcribe(copy, speech).is_ok(), Finish::Answered)
        });
        ensure!(
            matches!(ran, Ran::GaveUp),
            "the copy answered the long request before its client gave up"
        );
        services.pool.abandoned = Some(port);
        Ok(())
    })
    .await
}

#[when("the client sends its next request")]
async fn next_request(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let text = transcribe(&services.transcription_url(), short_speech()?)?;
        ensure!(!text.is_empty(), "the next request got an empty transcript");
        let abandoned = services.pool.abandoned.context("no request was given up")?;
        let other = *services
            .copy_ports
            .iter()
            .find(|port| **port != abandoned)
            .context("no other copy runs")?;
        services.pool.after_abandoned =
            Some((requests_taken(&[abandoned]), requests_taken(&[other])));
        Ok(())
    })
    .await
}

#[then("the other copy answers it, and the first copy gets a new request only after it answered the abandoned one")]
fn other_copy_answered(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let (abandoned, other) = services
        .pool
        .after_abandoned
        .context("no next request was sent")?;
    ensure!(
        abandoned == 1,
        "the copy of the abandoned request took {abandoned} requests while still transcribing it"
    );
    ensure!(
        other == 1,
        "the other copy took {other} requests, not the next one"
    );
    Ok(())
}

#[given("a third request waits and its client gives up")]
async fn third_request_gives_up(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let short: Vec<f32> = short_speech()?
            .into_iter()
            .map(|s| f32::from(s) / f32::from(i16::MAX))
            .collect();
        let gave_up = voicepuncher_command_spotting::confirm::UtteranceTranscriber::new(
            &services.transcription_url(),
        )?
        .transcribe(&short, AudioBuffer16k::SAMPLE_RATE_HZ, GIVES_UP_AFTER)
        .is_err();
        ensure!(
            gave_up,
            "the third request was answered although both copies were busy"
        );
        Ok(())
    })
    .await
}

/// How many requests the copies on `ports` took so far, from their logs.
fn requests_taken(ports: &[u16]) -> usize {
    ports
        .iter()
        .filter_map(|port| {
            std::fs::read_to_string(
                services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{port}.log")),
            )
            .ok()
        })
        .map(|text| text.matches(REQUEST_LOG_LINE).count())
        .sum()
}

#[when("the copies become free")]
async fn copies_free(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        answered(std::mem::take(&mut services.pool.long))?;
        Ok(())
    })
    .await
}

#[then("the abandoned request is not sent to any copy")]
fn abandoned_not_sent(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let taken = requests_taken(&services.copy_ports);
    ensure!(
        taken == BUSY_REQUESTS,
        "the copies took {taken} requests, not only the {BUSY_REQUESTS} long ones"
    );
    Ok(())
}
