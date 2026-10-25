//! The copies of a speech recognition service as its clients see them: the copy pool on a
//! probe told which copies answer, and the speech recognition client on fake copies.

use std::collections::HashSet;
use std::net::TcpListener;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use voicepuncher_copy_pool::{CopyPool, Finish, Probe, Ran, ServiceAddress};

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::{application::Transcriber, domain::AudioBuffer16k};

use super::fake_copies::{first_tried, free_copies, serve, FakeCopy};
use super::{local, lock};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

const SERVICE: &str = "http://127.0.0.1:8081/v1";
/// How often the copies are probed in the pool scenarios.
const PROBED_EVERY: Duration = Duration::from_millis(10);
const LEASE_WAIT: Duration = Duration::from_secs(2);
/// How long a slow probe takes to tell whether one port answers.
const SLOW_PROBE: Duration = Duration::from_millis(300);
/// "At once": well below one slow probe.
const AT_ONCE: Duration = Duration::from_millis(100);
/// The time limit of the request that waits on a copy that stopped answering.
const REQUEST_LIMIT: Duration = Duration::from_millis(10);
/// How long the copy that stopped answering takes over that request.
const STOPPED_FOR: Duration = Duration::from_millis(400);
/// How long the client waits for a copy to come back when none accepts connections.
const RECOVERY_WAIT: Duration = Duration::from_millis(300);
const CLIENTS: usize = 9;

/// What a copies scenario set up and saw.
#[derive(Debug, Default)]
pub(super) struct CopiesRun {
    base_url: String,
    /// Fake copies that answer.
    copies: Vec<FakeCopy>,
    /// Copies that accept nothing once the request is on its way.
    dying: Vec<TcpListener>,
    dying_ports: Vec<u16>,
    /// The copies' ports, as the copy pool scenarios name them.
    ports: Vec<u16>,
    /// The copies each request went to, in order.
    leased: Vec<u16>,
    /// What each request got back: its transcript, or why it failed.
    answers: Vec<Result<Option<String>, String>>,
    created_in: Option<Duration>,
    hung_seen: bool,
}

impl CopiesRun {
    /// The copies of the service at `base_url` on `listeners`, which accept nothing once
    /// the request is on its way.
    fn refusing(&mut self, base_url: String, listeners: Vec<TcpListener>) {
        self.base_url = base_url;
        self.dying_ports = listeners
            .iter()
            .filter_map(|l| l.local_addr().ok())
            .map(|a| a.port())
            .collect();
        self.dying = listeners;
    }
}

fn copies(world: &mut IntegrationWorld) -> &mut CopiesRun {
    pipeline_log();
    local(world).copies.get_or_insert_with(CopiesRun::default)
}

/// Answers on the ports it was given, at once or after `delay`.
#[derive(Clone)]
struct Answers {
    ports: Arc<Mutex<HashSet<u16>>>,
    delay: Duration,
}

impl Probe for Answers {
    fn answers(&self, port: u16) -> bool {
        std::thread::sleep(self.delay);
        lock(&self.ports).contains(&port)
    }
}

/// A pool of the copies on `ports` of the service, starting its search at `offset`.
fn pool(ports: &[u16], offset: usize, delay: Duration) -> Result<CopyPool> {
    let probe = Answers { ports: Arc::new(Mutex::new(ports.iter().copied().collect())), delay };
    Ok(CopyPool::discover(
        ServiceAddress::parse(SERVICE)?,
        probe,
        offset,
        PROBED_EVERY,
    ))
}

fn copy_ports(n: usize) -> Result<Vec<u16>> {
    Ok(ServiceAddress::parse(SERVICE)?.copy_ports()[..n].to_vec())
}

#[given("probing the copies of a service takes a while")]
fn slow_probe(world: &mut IntegrationWorld) {
    copies(world).ports = vec![8081];
}

#[when("a client of those copies is created")]
fn client_created(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let started = Instant::now();
    let pool = pool(&run.ports, 0, SLOW_PROBE)?;
    run.created_in = Some(started.elapsed());
    run.leased.push(pool.lease(LEASE_WAIT)?.port());
    Ok(())
}

#[then("it returns at once, and its first request goes to a copy that answers")]
fn returned_at_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let took = run.created_in.context("no client was created")?;
    ensure!(took < AT_ONCE, "creating the client took {took:?}");
    ensure!(
        run.leased == run.ports,
        "the request went to {:?}",
        run.leased
    );
    Ok(())
}

#[given("the speech recognition service runs as two copies, and one copy stops answering without exiting")]
fn one_copy_stops(world: &mut IntegrationWorld) -> Result<()> {
    copies(world).ports = copy_ports(2)?;
    Ok(())
}

#[when("a request waits on that copy past twice its time limit")]
fn request_waits_on_hung_copy(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let pool = pool(&run.ports, 0, Duration::ZERO)?;
    let lease = pool.lease(LEASE_WAIT)?;
    run.leased.push(lease.port());
    let hung_port = lease.port();
    let outcome = lease.run(REQUEST_LIMIT, |_| {
        std::thread::sleep(STOPPED_FOR);
        ((), Finish::Answered)
    });
    ensure!(matches!(outcome, Ran::GaveUp), "the copy answered in time");
    let deadline = Instant::now() + LEASE_WAIT;
    while !run.hung_seen && Instant::now() < deadline {
        run.hung_seen = pool
            .snapshot()
            .iter()
            .any(|copy| copy.port == hung_port && copy.hung);
        std::thread::sleep(PROBED_EVERY);
    }
    run.leased.push(pool.lease(AT_ONCE)?.port());
    Ok(())
}

#[then("that copy is logged as hung with the time waited, and the following requests go to the other copy")]
fn logged_hung(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let hung = *run.leased.first().context("no request was sent")?;
    ensure!(run.hung_seen, "copy {hung} was never counted hung");
    let wanted = format!("copy {hung} hung: no answer in ");
    let warnings = lock(&pipeline_log().warnings);
    ensure!(
        warnings.iter().any(|(_, line)| line.starts_with(&wanted)),
        "no line {wanted:?} in {warnings:?}"
    );
    ensure!(
        run.leased[1..].iter().all(|port| *port != hung),
        "{:?}",
        run.leased
    );
    Ok(())
}

#[given("the speech recognition service for dictation runs as three idle copies, and two processes whose process ids pick different copies")]
fn three_idle_copies(world: &mut IntegrationWorld) -> Result<()> {
    copies(world).ports = copy_ports(3)?;
    Ok(())
}

#[when("each process sends a request")]
fn each_process_sends(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    for offset in [0, 1] {
        // each process has its own pool, starting at the copy its process id picks
        let pool = pool(&run.ports, offset, Duration::ZERO)?;
        run.leased.push(pool.lease(LEASE_WAIT)?.port());
    }
    Ok(())
}

#[then("the requests go to two different copies")]
fn two_different_copies(world: &mut IntegrationWorld) -> Result<()> {
    let leased = &copies(world).leased;
    ensure!(
        leased.len() == 2 && leased[0] != leased[1],
        "the requests went to {leased:?}"
    );
    Ok(())
}

#[given("the speech recognition service for dictation runs as three copies")]
fn three_copies(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let (base_url, listeners) = free_copies(3)?;
    run.base_url = base_url;
    run.copies = listeners.into_iter().map(serve).collect();
    Ok(())
}

#[given("the speech recognition service runs as two copies, and the copy tried first exits as the request is about to reach it")]
fn first_copy_dies(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let (base_url, mut listeners) = free_copies(2)?;
    run.copies.push(serve(listeners.remove(1 - first_tried(2))));
    run.refusing(base_url, listeners);
    Ok(())
}

#[given("the speech recognition service runs as two copies, and none accepts connections")]
fn no_copy_accepts(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let (base_url, listeners) = free_copies(2)?;
    run.refusing(base_url, listeners);
    Ok(())
}

/// Half a second of audio, for a request of its own.
fn half_second() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![100i16; 8_000])
}

fn transcribed(transcriber: Result<OpenAiTranscriber>) -> Result<Option<String>, String> {
    let mut transcriber = transcriber.map_err(|e| format!("{e:#}"))?;
    let transcript = transcriber
        .transcribe(&half_second())
        .map_err(|e| format!("{e:#}"))?;
    Ok(transcript.map(|t| t.as_str().to_owned()))
}

fn client(base_url: &str) -> Result<OpenAiTranscriber> {
    let cfg = TranscriptionCfg { base_url: base_url.to_owned(), ..TranscriptionCfg::default() };
    Ok(OpenAiTranscriber::connect(cfg)?.with_recovery_wait(RECOVERY_WAIT))
}

#[when("a request is sent to the copies")]
fn request_sent(world: &mut IntegrationWorld) {
    let run = copies(world);
    let transcriber = client(&run.base_url);
    run.dying.clear();
    run.answers.push(transcribed(transcriber));
}

#[when("nine clients of one process send a request each at once")]
fn nine_clients(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let clients: Vec<_> = (0..CLIENTS)
        .map(|_| {
            let base_url = run.base_url.clone();
            std::thread::spawn(move || transcribed(client(&base_url)))
        })
        .collect();
    for client in clients {
        let answer = client
            .join()
            .map_err(|_| anyhow::anyhow!("a client panicked"))?;
        run.answers.push(answer);
    }
    Ok(())
}

#[then("each copy transcribes some of the requests, and no copy gets a second request before it answered the first")]
fn spread_over_copies(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    ensure!(
        run.answers
            .iter()
            .all(|a| a.as_ref().is_ok_and(|t| t.as_deref() == Some("hello"))),
        "{:?}",
        run.answers
    );
    for copy in &run.copies {
        ensure!(
            copy.requests() >= 1,
            "copy {} transcribed nothing",
            copy.port
        );
        ensure!(
            copy.most_at_once() == 1,
            "copy {} held two requests at once",
            copy.port
        );
    }
    let total: usize = run.copies.iter().map(FakeCopy::requests).sum();
    ensure!(total == CLIENTS, "{total} requests reached the copies");
    Ok(())
}

#[then(
    "the request is answered by the other copy, and the failed attempt on the dead copy is logged"
)]
fn answered_by_other(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let answer = run.answers.first().context("no request was sent")?;
    ensure!(
        answer.as_ref().is_ok_and(|t| t.as_deref() == Some("hello")),
        "{answer:?}"
    );
    let other = run.copies.first().context("no copy answers")?;
    ensure!(
        other.requests() == 1,
        "the other copy got {} requests",
        other.requests()
    );
    let dead = run.dying_ports.first().context("no copy died")?;
    let wanted = format!("copy {dead} refused the connection: skipped");
    let warnings = lock(&pipeline_log().warnings);
    ensure!(
        warnings.iter().any(|(_, line)| line.starts_with(&wanted)),
        "no line {wanted:?} in {warnings:?}"
    );
    Ok(())
}

#[then("the request fails once its attempts are used up, naming the copies it tried")]
fn fails_naming_copies(world: &mut IntegrationWorld) -> Result<()> {
    let run = copies(world);
    let answer = run.answers.first().context("no request was sent")?;
    let Err(message) = answer else {
        anyhow::bail!("the request was answered: {answer:?}");
    };
    for port in &run.dying_ports {
        ensure!(message.contains(&port.to_string()), "{message}");
    }
    Ok(())
}
