//! Steps of a hands-free session: starting it, speaking it and checking what it did.

use super::{
    bail, env, given, matches_kind, off_thread, pasted_as_said, session, then, when, CommandVoice,
    Context, Duration, E2eWorld, Event, Expected, HandsFreeSession, Instant, PathBuf, Result, Step,
    REACTION,
};
use voicepuncher::adapters::secondary::commandspotting::websocket::SpottingService;

pub(super) fn start_session(world: &mut E2eWorld) -> Result<()> {
    let url = world
        .voice
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let service = SpottingService { url, confirm_commands: !world.voice.unconfirmed_commands };
    let session = HandsFreeSession::start(service, env("VOICEPUNCHER_TRANSCRIPTION_URL")?)?;
    let deadline = Instant::now() + REACTION;
    while !session.events().contains(&Event::Listening) {
        if Instant::now() >= deadline {
            bail!("listening did not start");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    world.voice.session = Some(session);
    Ok(())
}

#[given("listening is on for a whole session")]
pub(super) fn listening_for_a_session(world: &mut E2eWorld) -> Result<()> {
    start_session(world)
}

#[given("the user's own recordings of the commands exist on the machine")]
pub(super) fn recordings_exist(_world: &mut E2eWorld) -> Result<()> {
    let dir = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
    if !dir.join("session-1").join("manifest.jsonl").is_file() {
        bail!("no recorded commands in {}", dir.display());
    }
    Ok(())
}

pub(super) fn session_of(world: &E2eWorld) -> Result<&HandsFreeSession> {
    world.voice.session.as_ref().context("listening is not on")
}

#[when("a synthesized voice goes through the hands-free session")]
pub(super) async fn synthesized_session(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        session_of(world)?.run(CommandVoice::Synthesized)
    })
    .await
}

#[when("the hands-free session is spoken with the user's recorded commands")]
pub(super) async fn recorded_session(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        session_of(world)?.run(CommandVoice::Recorded { long_pauses: false })
    })
    .await
}

#[when("the hands-free session is spoken with the user's recorded commands and long pauses")]
pub(super) async fn recorded_session_with_pauses(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        session_of(world)?.run(CommandVoice::Recorded { long_pauses: true })
    })
    .await
}

#[then("the app does everything the session said, in order")]
pub(super) fn session_done(world: &mut E2eWorld) -> Result<()> {
    let events: Vec<Event> = session_of(world)?
        .events()
        .into_iter()
        .filter(|e| *e != Event::Listening)
        .collect();
    let expected: Vec<Expected> = session()
        .into_iter()
        .filter_map(|step| match step {
            Step::Command(_, reaction) => Some(reaction),
            Step::Dictate(..) => None,
        })
        .collect();
    let in_order = events.len() == expected.len() && events.iter().zip(&expected).all(is_expected);
    if !in_order {
        bail!("expected {expected:?}\n     got {events:?}");
    }
    Ok(())
}

/// Whether `event` is what the session said: a paste of its sentence, else its kind.
fn is_expected((event, wanted): (&Event, &Expected)) -> bool {
    match (event, wanted) {
        (Event::Pasted(text), Expected::Pasted(sentence)) => pasted_as_said(text, sentence),
        _ => matches_kind(event, wanted),
    }
}
