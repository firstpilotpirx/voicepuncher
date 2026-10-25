//! Steps of a request the client gave up on: the next one goes to another copy and an abandoned one is never sent.

use super::{
    answered, anyhow, client_requests, ensure, given, long_speech, off_thread, services,
    services_dir, short_speech, then, transcribe, when, AudioBuffer16k, Context, CopyPool,
    E2eWorld, Finish, Ran, Result, ABANDONED_SPEECH_REPEATS, BUSY_REQUESTS, BUSY_WITHIN,
    GIVES_UP_AFTER, SPEECH_RECOGNITION,
};

#[given("a client gave up on a request that one copy is still transcribing")]
pub(super) async fn gave_up_on_a_request(world: &mut E2eWorld) -> Result<()> {
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
pub(super) async fn next_request(world: &mut E2eWorld) -> Result<()> {
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
pub(super) fn other_copy_answered(world: &mut E2eWorld) -> Result<()> {
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
pub(super) async fn third_request_gives_up(world: &mut E2eWorld) -> Result<()> {
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

/// The length in samples of the audio of every request each copy took, in the order taken.
pub(super) fn requests_taken_by_copy(ports: &[u16]) -> Vec<Vec<usize>> {
    ports
        .iter()
        .map(|port| {
            let log = std::fs::read_to_string(
                services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{port}.log")),
            )
            .unwrap_or_default();
            client_requests(&log)
                .map(|line| {
                    line.split_once("' (")
                        .and_then(|(_, rest)| rest.split(' ').next())
                        .and_then(|samples| samples.parse().ok())
                        .unwrap_or_default()
                })
                .collect()
        })
        .collect()
}

/// How many requests the copies on `ports` took so far, from their logs.
pub(super) fn requests_taken(ports: &[u16]) -> usize {
    ports
        .iter()
        .filter_map(|port| {
            std::fs::read_to_string(
                services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{port}.log")),
            )
            .ok()
        })
        .map(|text| client_requests(&text).count())
        .sum()
}

#[when("the copies become free")]
pub(super) async fn copies_free(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        answered(std::mem::take(&mut services.pool.long))?;
        Ok(())
    })
    .await
}

#[then("the abandoned request is not sent to any copy")]
pub(super) fn abandoned_not_sent(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let taken = requests_taken(&services.copy_ports);
    ensure!(
        taken == BUSY_REQUESTS,
        "the copies took {taken} requests, not only the {BUSY_REQUESTS} long ones"
    );
    Ok(())
}
