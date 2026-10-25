//! Steps of `local_services.feature` about a service that hangs without exiting: the
//! scripts restart a command spotting service that stops answering its health check, and
//! leave one that is merely slow alone.

pub(super) mod confirming;

use std::io::Read;
use std::net::TcpListener;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use super::{healthy, services, wait_until, ScenarioServices, COMMAND_SPOTTING};
use crate::steps::off_thread;
use crate::E2eWorld;

/// How many seconds apart the scripts check the scenario's services: short, so the scenarios
/// run through several checks in seconds instead of the default 5 s cadence.
const CHECK_EVERY_SECS: u32 = 1;
/// Long enough after its start for the scripts to have seen the service answer: three checks.
const ANSWERED_ONCE_AFTER: Duration = Duration::from_secs(3);
const RESTART_WITHIN: Duration = Duration::from_secs(30);
/// What the supervisor writes when it restarts a service that stopped answering.
const HEALTH_RESTART_NOTE: &str = "stopped answering its health check";
/// How late the slow speech recognition service answers every confirmation.
const SLOW_ANSWER: Duration = Duration::from_secs(4);
/// Through six checks: twice the checks in a row that restart a service.
const STREAM_FOR: Duration = Duration::from_secs(6);
const FRAME_SAMPLES: usize = 1_600;
const FRAME: Duration = Duration::from_millis(100);
const SPEECH: &str =
    "so I was saying that the weather is nice today and we could go for a walk later on";
const HEARD: &str = r#"{"text":" the weather is nice","segments":[]}"#;

#[given("the service scripts started the command spotting service")]
async fn command_spotting_started(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::for_outages()?;
        services.check_health_every(CHECK_EVERY_SECS);
        services.start(&[COMMAND_SPOTTING])?;
        world.services.local_services = Some(services);
        Ok(())
    })
    .await
}

/// A speech recognition service for commands on `listener` that answers every request
/// `SLOW_ANSWER` late.
fn slow_speech_recognition(listener: TcpListener) {
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut buf = vec![0u8; 65536];
                let _ = stream.read(&mut buf);
                std::thread::sleep(SLOW_ANSWER);
                let _ = std::io::Write::write_all(
                    &mut stream,
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{HEARD}",
                        HEARD.len()
                    )
                    .as_bytes(),
                );
            });
        }
    });
}

#[when("its process stops answering without exiting")]
async fn service_hangs(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        std::thread::sleep(ANSWERED_ONCE_AFTER);
        let services = services(world)?;
        let pid = services
            .command_spotting_process()
            .context("no command spotting service process was found after its start")?;
        services.pause(pid)?;
        services.watched.hung = Some(pid);
        Ok(())
    })
    .await
}

#[when("an app streams speech to it through several of the scripts' checks while the speech recognition service for commands answers its confirmations slowly")]
async fn stream_through_several_checks(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        slow_speech_recognition(TcpListener::bind(("127.0.0.1", services.command_stt_port))?);
        services.watched.hung = services.command_spotting_process();
        let speech = speech_as_f32()?;
        let mut client = SpottingClient::connect(&services.command_spotting_url(), Instant::now())?;
        client.listen_for(&voicepuncher::core::domain::VoiceCommand::STANDBY)?;
        let streamed = stream_for(&mut client, &speech);
        client.close();
        streamed
    })
    .await
}

/// Streams `speech` over and over, in real time, for [`STREAM_FOR`].
fn stream_for(client: &mut SpottingClient, speech: &[f32]) -> Result<()> {
    let started = Instant::now();
    for frame in speech.chunks(FRAME_SAMPLES).cycle() {
        if started.elapsed() >= STREAM_FOR {
            break;
        }
        client.send_audio(frame)?;
        client.keep_alive()?;
        client.read_events()?;
        std::thread::sleep(FRAME);
    }
    Ok(())
}

fn speech_as_f32() -> Result<Vec<f32>> {
    let samples = synthesized(&SayRequest { voice: None, words_per_minute: None, text: SPEECH })?;
    Ok(samples
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect())
}

#[then("the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering")]
async fn restarted_after_hang(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let hung = services
            .watched
            .hung
            .context("no process was made to hang")?;
        let port = services.kws_port();
        let came_back = wait_until(RESTART_WITHIN, || {
            healthy(port)
                && services
                    .command_spotting_process()
                    .is_some_and(|pid| pid != hung)
        });
        ensure!(
            came_back,
            "the hung command spotting service did not answer again within {RESTART_WITHIN:?}"
        );
        let log = std::fs::read_to_string(services.command_spotting_log())?;
        ensure!(
            log.contains(HEALTH_RESTART_NOTE),
            "its log does not note why it was restarted"
        );
        Ok(())
    })
    .await
}

#[then("the service is not restarted")]
fn not_restarted(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let log = std::fs::read_to_string(services.command_spotting_log())?;
    ensure!(
        !log.contains(HEALTH_RESTART_NOTE),
        "the busy service was restarted:\n{log}"
    );
    ensure!(
        services.watched.hung.is_some()
            && services.command_spotting_process() == services.watched.hung,
        "the command spotting service process changed"
    );
    Ok(())
}
