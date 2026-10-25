//! Steps of a copy that dies: the supervisor brings it back and no copy outlives the service.

use super::{
    alive, answers, any_process, copy_list, ensure, ensure_all_answered, given, off_thread, pgrep,
    services, started_with, then, unanswered_at_once, wait_until, when, Command, Context, E2eWorld,
    Instant, Result, COPY_BACK_WITHIN, PROCESS_GONE_WITHIN, REQUESTS_WHILE_DOWN, SEVERAL_COPIES,
    SPEECH_RECOGNITION,
};

#[given(
    "the service scripts started the speech recognition service for dictation without a number of copies"
)]
pub(super) async fn started_by_default(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, None)).await
}

#[given("the service scripts started the speech recognition service as several copies")]
pub(super) async fn started_as_copies(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(SEVERAL_COPIES))).await
}

#[when("one copy exits on its own")]
pub(super) async fn copy_dies(world: &mut E2eWorld) -> Result<()> {
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

#[then("the service keeps answering transcription requests, and that copy answers again within 30 seconds")]
pub(super) async fn copy_came_back(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = *services.copy_ports.first().context("no copies")?;
        let killed = services.pool.copy_killed.context("no copy was killed")?;
        let down = !answers(port);
        let unanswered =
            unanswered_at_once(&services.transcription_url(), REQUESTS_WHILE_DOWN, None)?;
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
pub(super) async fn no_copy_left(world: &mut E2eWorld) -> Result<()> {
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
