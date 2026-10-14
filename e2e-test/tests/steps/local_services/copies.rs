//! Steps of `local_services.feature` about the speech recognition service run as several
//! copies behind one address: the copies share the requests, one that dies comes back,
//! short speech goes first, and stopping leaves nothing behind.

use std::process::Command;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::Transcriber;
use voicepuncher::core::domain::AudioBuffer16k;
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
const COPY_BACK_WITHIN: Duration = Duration::from_secs(30);
const PROCESS_GONE_WITHIN: Duration = Duration::from_secs(10);
/// Long speech: two keep both copies busy, two wait in the queue.
const LONG_REQUESTS: usize = 4;
/// The first long request that had to wait for a copy.
const FIRST_WAITING: usize = 2;
/// Requests sent at once right after a copy died: more than the copies that still answer.
const REQUESTS_WHILE_DOWN: usize = 4;
/// Between two requests sent "one after another", so they reach the queue in that order.
const BETWEEN_REQUESTS: Duration = Duration::from_millis(200);
/// What a copy logs for every request it transcribes.
const TRANSCRIBED_LOG_LINE: &str = "whisper_vad: detected";
const SHORT_UTTERANCE: &str = "press enter";

/// The long requests of the queue scenario, each ending with the moment it was answered,
/// and the moment the short utterance was answered.
#[derive(Debug, Default)]
pub struct PoolRun {
    long: Vec<JoinHandle<Result<Instant>>>,
    short_answered: Option<Instant>,
    answered: Option<usize>,
    copy_killed: Option<Instant>,
    taken: Option<TakenPort>,
}

impl ScenarioServices {
    /// The ports of the copies the service scripts started, from the list they keep.
    pub(super) fn running_copy_ports(&self) -> Vec<u16> {
        std::fs::read_to_string(copy_list(self.stt_port))
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|port| port.parse().ok())
            .collect()
    }
}

fn copy_list(stt_port: u16) -> std::path::PathBuf {
    services_dir("run").join(format!("{SPEECH_RECOGNITION}-{stt_port}.copies"))
}

fn balancer_config(stt_port: u16) -> std::path::PathBuf {
    services_dir("run").join(format!("haproxy-{stt_port}.cfg"))
}

/// The load balancer in front of the copies on `stt_port`, found by its configuration.
pub(super) fn balancer_process(stt_port: u16) -> Option<u32> {
    pgrep(&format!("haproxy .*haproxy-{stt_port}\\.cfg"))
}

pub(super) fn remove_balancer_files(stt_port: u16) {
    let _ = std::fs::remove_file(copy_list(stt_port));
    let _ = std::fs::remove_file(balancer_config(stt_port));
    for (dir, ext) in [("run", "pid"), ("logs", "log")] {
        let _ = std::fs::remove_file(services_dir(dir).join(format!("haproxy-{stt_port}.{ext}")));
    }
}

/// The transcript the service at `url` gives for `samples`, through the app's own client.
fn transcribe(url: &str, samples: Vec<i16>) -> Result<String> {
    let mut client = OpenAiTranscriber::connect(TranscriptionCfg {
        base_url: url.to_owned(),
        ..TranscriptionCfg::default()
    })?;
    let transcript = client.transcribe(&AudioBuffer16k::new(samples))?;
    Ok(transcript
        .map(|t| t.as_str().to_owned())
        .unwrap_or_default())
}

/// How many of `count` short requests sent at once to `url` got a transcript.
fn answered_at_once(url: &str, count: usize) -> Result<usize> {
    let speech = short_speech()?;
    let requests: Vec<_> = (0..count)
        .map(|_| {
            let (url, speech) = (url.to_owned(), speech.clone());
            std::thread::spawn(move || transcribe(&url, speech))
        })
        .collect();
    Ok(requests
        .into_iter()
        .filter_map(|request| request.join().ok()?.ok())
        .filter(|text| !text.is_empty())
        .count())
}

fn short_speech() -> Result<Vec<i16>> {
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

#[given("the service scripts started the speech recognition service without a number of copies")]
async fn started_by_default(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, None)).await
}

#[given("the service scripts started the speech recognition service as several copies")]
async fn started_as_copies(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(SEVERAL_COPIES))).await
}

#[given("the speech recognition service runs as two copies, both busy with long speech, and more long speech waits")]
async fn copies_busy(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        started_with(world, Some(SEVERAL_COPIES))?;
        let services = services(world)?;
        let url = services.transcription_url();
        let speech = long_speech()?;
        for _ in 0..LONG_REQUESTS {
            let (url, speech) = (url.clone(), speech.clone());
            services.pool.long.push(std::thread::spawn(move || {
                transcribe(&url, speech)?;
                Ok(Instant::now())
            }));
            std::thread::sleep(BETWEEN_REQUESTS);
        }
        Ok(())
    })
    .await
}

#[when("several transcription requests arrive at once")]
async fn requests_at_once(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        services.pool.answered = Some(answered_at_once(
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

#[when("a short utterance arrives")]
async fn short_utterance(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        transcribe(&services.transcription_url(), short_speech()?)?;
        services.pool.short_answered = Some(Instant::now());
        Ok(())
    })
    .await
}

#[then("every request gets its transcript, and each of the three copies transcribed some of them")]
fn shared_by_copies(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let answered = services.pool.answered.context("no requests were sent")?;
    ensure!(
        answered == REQUESTS_AT_ONCE,
        "{answered} of {REQUESTS_AT_ONCE} requests got a transcript"
    );
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
        let answered = answered_at_once(&services.transcription_url(), REQUESTS_WHILE_DOWN)?;
        ensure!(
            answered == REQUESTS_WHILE_DOWN,
            "{answered} of {REQUESTS_WHILE_DOWN} requests got a transcript while a copy was down (it was {})",
            if down { "down when they were sent" } else { "already back" }
        );
        let back_within = COPY_BACK_WITHIN.saturating_sub(killed.elapsed());
        ensure!(
            wait_until(back_within, || answers(port)),
            "the copy on port {port} did not answer within {COPY_BACK_WITHIN:?} of its exit"
        );
        Ok(())
    })
    .await
}

#[then("the short utterance is transcribed before the long speech that waited")]
async fn short_went_first(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let short = services
            .pool
            .short_answered
            .context("no short utterance was sent")?;
        let mut long = Vec::new();
        for request in services.pool.long.drain(..) {
            long.push(request.join().expect("a long request thread panicked")?);
        }
        let waited = *long
            .get(FIRST_WAITING)
            .context("too little long speech was sent")?;
        ensure!(
            short < waited,
            "the short utterance was answered {:?} after the long speech that waited",
            short.duration_since(waited)
        );
        Ok(())
    })
    .await
}

#[then("nothing answers on its port, and no copy of it is left")]
async fn no_copy_left(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        ensure!(
            !answers(services.stt_port),
            "the speech recognition service still answers after it was stopped"
        );
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
        let stt_port = services.stt_port;
        ensure!(
            wait_until(PROCESS_GONE_WITHIN, || balancer_process(stt_port).is_none()),
            "the load balancer in front of the copies is left"
        );
        ensure!(
            !copy_list(stt_port).exists() && !balancer_config(stt_port).exists(),
            "the list of copies or the load balancer's configuration is left"
        );
        Ok(())
    })
    .await
}

/// The ports the service scripts give the copies of the service on `stt_port`: 10000 above it,
/// or 10000 below when that would pass the last port.
fn expected_copy_ports(stt_port: u16, copies: u32) -> Vec<u32> {
    let port = u32::from(stt_port);
    let base = if port + 10_000 + copies <= u32::from(u16::MAX) {
        port + 10_000
    } else {
        port - 10_000
    };
    (1..=copies).map(|i| base + i).collect()
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

#[then("the start fails naming the service and the log of the copy that failed, and neither the copy that started nor anything in front of it is left")]
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
            balancer_process(stt_port).is_none() && !copy_list(stt_port).exists(),
            "the load balancer or the list of copies is left after the failed start"
        );
        Ok(())
    })
    .await
}

/// How long the client of the abandoned request waits before it gives up on it.
const GIVES_UP_AFTER: Duration = Duration::from_millis(300);
/// The long requests that keep both copies busy, each this many times the long speech.
const BUSY_REQUESTS: usize = 2;
/// Long enough that both copies are still busy when the third client gives up.
const LONG_SPEECH_REPEATS: usize = 10;
/// How long after the copies become free the abandoned request is given up: at once.
const GIVEN_UP_WITHIN: Duration = Duration::from_secs(2);
/// What a copy logs when it gives up a request whose client is gone.
const ABORTED_LOG_LINE: &str = "client disconnected, aborted processing";
/// How long the long requests may take to reach both copies.
const BUSY_WITHIN: Duration = Duration::from_secs(20);
/// What a copy logs for every request it takes, with the length of its audio.
const REQUEST_LOG_LINE: &str = "processing '";

#[given("the speech recognition service runs as two copies, both busy with long speech, and a third request waits and its client gives up")]
async fn abandoned_request(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        started_with(world, Some(SEVERAL_COPIES))?;
        let services = services(world)?;
        let url = services.transcription_url();
        let speech: Vec<i16> = long_speech()?.repeat(LONG_SPEECH_REPEATS);
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
        let short: Vec<f32> = short_speech()?
            .into_iter()
            .map(|s| f32::from(s) / f32::from(i16::MAX))
            .collect();
        let gave_up = voicepuncher_command_spotting::confirm::UtteranceTranscriber::new(&url)
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
        for request in services.pool.long.drain(..) {
            request
                .join()
                .map_err(|_| anyhow::anyhow!("a long request panicked"))??;
        }
        Ok(())
    })
    .await
}

#[then("the copy that takes the abandoned request gives it up at once instead of transcribing it")]
fn abandoned_given_up(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let aborted_lines = || -> usize {
        services
            .copy_ports
            .iter()
            .filter_map(|port| {
                std::fs::read_to_string(
                    services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{port}.log")),
                )
                .ok()
            })
            .map(|text| text.matches(ABORTED_LOG_LINE).count())
            .sum::<usize>()
    };
    wait_until(GIVEN_UP_WITHIN, || aborted_lines() >= 1);
    let aborted = aborted_lines();
    ensure!(
        aborted == 1,
        "{aborted} requests were given up by a copy, not the one abandoned request"
    );
    Ok(())
}
