//! Steps of a client finding the copies from the service's address.

use super::{
    answers, ensure, found_copies, given, off_thread, services, short_speech, started_with, then,
    transcribe, when, Context, E2eWorld, Result, SEVERAL_COPIES, THREE_COPIES,
};

#[given("the service scripts started the speech recognition service for dictation as three copies")]
pub(super) async fn started_as_three(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(THREE_COPIES))).await
}

#[given("the service scripts started the speech recognition service for dictation as one copy")]
pub(super) async fn started_as_one(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(1))).await
}

#[given("the speech recognition service for dictation runs as two copies")]
pub(super) async fn started_as_two(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| started_with(world, Some(SEVERAL_COPIES))).await
}

#[when("a client connects to the service by its address")]
pub(super) async fn client_connects(world: &mut E2eWorld) -> Result<()> {
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
pub(super) fn found_three(world: &mut E2eWorld) -> Result<()> {
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
pub(super) fn found_one(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let found = services.pool.found.clone().context("no client connected")?;
    ensure!(
        found == [services.stt_port],
        "the client found the copies {found:?}, not the service's own port {}",
        services.stt_port
    );
    Ok(())
}
