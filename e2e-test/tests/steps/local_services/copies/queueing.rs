//! Steps of requests beyond the free copies: they wait their turn.

use super::{
    anyhow, ensure, given, long_speech, off_thread, requests_taken, requests_taken_by_copy,
    services, short_speech, started_with, then, transcribe, wait_until, when, Context, E2eWorld,
    Instant, JoinHandle, Result, BETWEEN_REQUESTS, BUSY_REQUESTS, BUSY_SPEECH_REPEATS, BUSY_WITHIN,
    LOG_POLL, MORE_REQUESTS, PADDING_SAMPLES, REACHED_WITHIN, SEVERAL_COPIES,
};

#[given(
    "the speech recognition service for dictation runs as two copies, both busy with long speech"
)]
pub(super) async fn copies_busy(world: &mut E2eWorld) -> Result<()> {
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

/// When each new request reached one of the copies on `ports`, with its audio length, as
/// their logs tell, until [`MORE_REQUESTS`] did.
fn watch_reached(ports: &[u16]) -> Vec<(Instant, usize)> {
    let mut seen = requests_taken_by_copy(ports);
    let mut reached = Vec::new();
    let started = Instant::now();
    while reached.len() < MORE_REQUESTS && started.elapsed() < REACHED_WITHIN {
        let taken_now = requests_taken_by_copy(ports);
        let new = taken_now
            .iter()
            .zip(&seen)
            .flat_map(|(taken, before)| taken.iter().skip(before.len()));
        reached.extend(new.map(|samples| (Instant::now(), *samples)));
        seen = taken_now;
        std::thread::sleep(LOG_POLL);
    }
    reached
}

#[when("two more requests arrive one after another")]
pub(super) async fn more_requests(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let url = services.transcription_url();
        let speech = short_speech()?;
        let ports = services.copy_ports.clone();
        services.pool.reached = Some(std::thread::spawn(move || watch_reached(&ports)));
        for n in 0..MORE_REQUESTS {
            let url = url.clone();
            let mut speech = speech.clone();
            speech.resize(speech.len() + n * PADDING_SAMPLES, 0);
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
pub(super) fn answered(requests: Vec<JoinHandle<Result<Instant>>>) -> Result<Vec<Instant>> {
    requests
        .into_iter()
        .map(|request| {
            request
                .join()
                .map_err(|_| anyhow!("a request thread panicked"))?
        })
        .collect()
}

/// Checks that the n-th waiting request reached a copy only once n copies freed: a long
/// request answered, or an earlier waiting one; and that they started in the order they came.
fn ensure_in_turn(reached: &[(Instant, usize)], long: &[Instant], more: &[Instant]) -> Result<()> {
    let mut freed: Vec<Instant> = long.iter().chain(more).copied().collect();
    freed.sort();
    for (n, (at, _)) in reached.iter().enumerate() {
        ensure!(
            *at >= freed[n],
            "request {} reached a copy {:?} before a copy was free for it: a copy took two requests at once",
            BUSY_REQUESTS + n + 1,
            freed[n] - *at
        );
    }
    // Each request after the first carries more audio than the one before it
    let mut by_arrival = reached.to_vec();
    by_arrival.sort_by_key(|(_, samples)| *samples);
    ensure!(
        by_arrival.windows(2).all(|pair| pair[0].1 < pair[1].1),
        "the copies' logs do not tell the waiting requests apart by their audio: {reached:?}"
    );
    ensure!(
        by_arrival.windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "the second request started on a copy before the first one"
    );
    Ok(())
}

#[then("each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once")]
pub(super) async fn waited_their_turn(world: &mut E2eWorld) -> Result<()> {
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
        ensure_in_turn(&reached, &long, &more)?;
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
