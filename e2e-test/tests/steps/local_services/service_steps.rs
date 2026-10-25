//! Steps of the service scripts: starting, killing, stopping and restarting the services.

use super::{
    alive, answers, any_process, bail, ensure, given, off_thread, then, wait_until, when, Command,
    Context, E2eWorld, Result, ScenarioServices, BROKEN_MODEL_LOG_LINE, PROCESS_GONE_WITHIN,
    RESTART_NOTE, RESTART_WITHIN, SERVICE_STARTUP_OUTPUT, SPEECH_RECOGNITION, START_FAILURE_WITHIN,
    STAYS_STOPPED_FOR,
};

pub(super) fn services(world: &mut E2eWorld) -> Result<&mut ScenarioServices> {
    world
        .services
        .local_services
        .as_mut()
        .context("no services were set up by a Given step")
}

#[given("no local service runs on the chosen ports")]
pub(super) fn free_ports(world: &mut E2eWorld) -> Result<()> {
    world.services.local_services = Some(ScenarioServices::on_free_ports()?);
    Ok(())
}

#[given("the service scripts started the speech recognition service")]
pub(super) async fn speech_recognition_started(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::on_free_ports()?;
        services.start_speech_recognition()?;
        let started = services.run.started.as_ref().expect("just started it");
        if !started.status.success() {
            bail!(
                "the service scripts could not start the speech recognition service:\n{}{}",
                String::from_utf8_lossy(&started.stdout),
                String::from_utf8_lossy(&started.stderr)
            );
        }
        world.services.local_services = Some(services);
        Ok(())
    })
    .await
}

#[given("the speech recognition model file is not a model")]
pub(super) fn broken_speech_model(world: &mut E2eWorld) -> Result<()> {
    let mut services = ScenarioServices::on_free_ports()?;
    services.break_speech_model()?;
    world.services.local_services = Some(services);
    Ok(())
}

#[when(
    regex = r"^(?:the user starts only the speech recognition service for dictation|the service scripts start the speech recognition service)(?: with the service scripts)?$"
)]
pub(super) async fn start_speech_recognition(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| services(world)?.start_speech_recognition()).await
}

#[when("its process exits on its own")]
pub(super) async fn service_dies(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let pid = services
            .run
            .service
            .context("no speech recognition service process was found after its start")?;
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
        wait_until(PROCESS_GONE_WITHIN, || !alive(pid));
        Ok(())
    })
    .await
}

#[when("the user stops it with the service scripts")]
pub(super) async fn user_stops_service(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        services(world)?.just(&["stop-model-servers", SPEECH_RECOGNITION])?;
        Ok(())
    })
    .await
}

#[then("the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands and the command spotting service")]
pub(super) fn only_speech_recognition_answers(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let started = services.started()?;
    ensure!(
        started.status.success(),
        "starting only the speech recognition service failed:\n{}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );
    ensure!(
        answers(services.stt_port),
        "the speech recognition service for dictation does not answer"
    );
    ensure!(
        !answers(services.command_stt_port),
        "a speech recognition service for commands was started too"
    );
    ensure!(
        !answers(services.kws_port),
        "a command spotting service was started too"
    );
    Ok(())
}

#[then("the service answers on its port again within 30 seconds, and its log keeps the output of the run that ended followed by a note of the restart")]
pub(super) async fn service_restarted(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = services.stt_port;
        ensure!(
            wait_until(RESTART_WITHIN, || answers(port)),
            "the speech recognition service did not answer within {RESTART_WITHIN:?} after its process exited"
        );
        let log = std::fs::read_to_string(services.log_file())
            .with_context(|| format!("log {}", services.log_file().display()))?;
        let note = log
            .find(RESTART_NOTE)
            .context("the log has no note of the restart")?;
        ensure!(
            log[..note].contains(SERVICE_STARTUP_OUTPUT),
            "the log lost the output of the run that ended"
        );
        Ok(())
    })
    .await
}

#[then("nothing answers on its port, still 5 seconds later, and no process of it is left")]
pub(super) async fn service_stays_stopped(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = services.stt_port;
        ensure!(
            !answers(port),
            "the service still answers after it was stopped"
        );
        std::thread::sleep(STAYS_STOPPED_FOR);
        ensure!(
            !answers(port),
            "the service answers again {STAYS_STOPPED_FOR:?} after it was stopped"
        );
        ensure!(
            any_process(port).is_none(),
            "a process of the stopped service is left: {:?}",
            any_process(port)
        );
        Ok(())
    })
    .await
}

#[then("the start fails within 15 seconds naming the service and its log, and no process of it is left")]
pub(super) async fn start_failed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let started = services.started()?;
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&started.stdout),
            String::from_utf8_lossy(&started.stderr)
        );
        ensure!(
            !started.status.success(),
            "the start of a service with a broken model succeeded:\n{said}"
        );
        let took = services.run.start_took.context("the start was not timed")?;
        ensure!(
            took <= START_FAILURE_WITHIN,
            "the start failed only after {took:?}, not within {START_FAILURE_WITHIN:?}"
        );
        check_failure_report(
            &String::from_utf8_lossy(&started.stderr),
            &services.log_file(),
        )?;
        let port = services.stt_port;
        ensure!(
            wait_until(PROCESS_GONE_WITHIN, || any_process(port).is_none()),
            "a process of the service that failed to start is left: {:?}",
            any_process(port)
        );
        ensure!(
            !services.pid_file().exists(),
            "the PID file of the service that failed to start is left"
        );
        Ok(())
    })
    .await
}

/// Fails unless the failure `report` names the service, its `log` and the log's last lines.
fn check_failure_report(report: &str, log: &std::path::Path) -> Result<()> {
    ensure!(
        report.contains(SPEECH_RECOGNITION) && report.contains(&*log.to_string_lossy()),
        "the failure report does not name {SPEECH_RECOGNITION} and its log {}:\n{report}",
        log.display()
    );
    ensure!(
        report.contains(BROKEN_MODEL_LOG_LINE),
        "the failure report does not show the service's last log lines:\n{report}"
    );
    Ok(())
}
