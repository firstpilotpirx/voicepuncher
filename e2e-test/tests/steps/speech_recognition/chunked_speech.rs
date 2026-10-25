//! Steps of long speech transcribed in chunks: no repeated, missing or dropped words.

use super::{
    bail, dropped_phrase_len, given, missing_word_share, repeated_phrases, synthesize, then,
    write_wav, Condition, Context, E2eWorld, Lang, Regex, Result, MAX_DROPPED_PHRASE_WORDS,
    MAX_MISSING_WORD_SHARE,
};

#[given(regex = r"^a recording file of (Russian|English) speech longer than the chunk threshold$")]
pub(super) fn long_speech_recording(world: &mut E2eWorld, language: String) -> Result<()> {
    record_speech(world, language_of(&language)?, Condition::Normal, 0.0)
}

#[then("the recording is transcribed in chunks")]
pub(super) fn transcribed_in_chunks(world: &mut E2eWorld) -> Result<()> {
    let chunks = Regex::new(r"file mode: (\d+) chunks")?
        .captures(&world.app.stderr)
        .and_then(|c| c[1].parse::<usize>().ok());
    match chunks {
        Some(n) if n >= 2 => Ok(()),
        _ => bail!(
            "the recording was not transcribed in chunks — stderr:\n{}",
            world.app.stderr
        ),
    }
}

#[then("the printed text repeats no phrase of the speech")]
pub(super) fn no_repeated_phrase(world: &mut E2eWorld) -> Result<()> {
    let spoken = world.spoken.context("no speech was recorded")?.text();
    let repeated = repeated_phrases(spoken, &world.app.stdout);
    if !repeated.is_empty() {
        bail!(
            "phrases repeated at a chunk seam: {repeated:?}\n  {}",
            world.printed_and_log()
        );
    }
    Ok(())
}

#[then("the printed text misses at most one spoken word in ten")]
pub(super) fn few_missing_words(world: &mut E2eWorld) -> Result<()> {
    let spoken = world.spoken.context("no speech was recorded")?.text();
    let missing = missing_word_share(spoken, &world.app.stdout);
    if missing > MAX_MISSING_WORD_SHARE {
        bail!(
            "{:.0} % of the spoken words are missing\n  {}",
            missing * 100.0,
            world.printed_and_log()
        );
    }
    Ok(())
}

#[then("no run of more than five consecutive spoken words is missing from the printed text")]
pub(super) fn no_dropped_phrase(world: &mut E2eWorld) -> Result<()> {
    let spoken = world.spoken.context("no speech was recorded")?.text();
    let dropped = dropped_phrase_len(spoken, &world.app.stdout);
    if dropped > MAX_DROPPED_PHRASE_WORDS {
        bail!(
            "a phrase of {dropped} spoken words is missing\n  {}",
            world.printed_and_log()
        );
    }
    Ok(())
}

#[given(
    regex = r"^a recording file of (Russian|English) speech longer than the chunk threshold, ([^,]+)$"
)]
pub(super) fn long_speech_under_condition(
    world: &mut E2eWorld,
    language: String,
    condition: String,
) -> Result<()> {
    record_speech(
        world,
        language_of(&language)?,
        condition_of(&condition)?,
        0.0,
    )
}

#[given(
    regex = r"^a recording file of (Russian|English) speech longer than the chunk threshold, ([^,]+), after a lead-in of ([0-9.]+) seconds$"
)]
pub(super) fn long_speech_after_lead_in(
    world: &mut E2eWorld,
    language: String,
    condition: String,
    lead_in_secs: f32,
) -> Result<()> {
    record_speech(
        world,
        language_of(&language)?,
        condition_of(&condition)?,
        lead_in_secs,
    )
}

pub(super) fn condition_of(name: &str) -> Result<Condition> {
    CONDITIONS
        .iter()
        .find(|(said, _)| *said == name)
        .map(|(_, condition)| *condition)
        .ok_or_else(|| anyhow::anyhow!("no recording condition '{name}'"))
}

/// Each recording condition as a scenario names it.
const CONDITIONS: [(&str, Condition); 11] = [
    ("spoken at a normal pace", Condition::Normal),
    ("spoken slowly", Condition::Slow),
    ("spoken fast", Condition::Fast),
    ("with pauses between sentences", Condition::SentencePauses),
    ("with pauses between words", Condition::WordPauses),
    ("with low background noise", Condition::NoiseLow),
    ("with high background noise", Condition::NoiseHigh),
    ("through a phone-quality line", Condition::PhoneBand),
    ("spoken quietly", Condition::Quiet),
    ("clipped", Condition::Clipped),
    ("with short audio dropouts", Condition::Dropouts),
];

pub(super) fn language_of(name: &str) -> Result<Lang> {
    match name {
        "Russian" => Ok(Lang::Ru),
        "English" => Ok(Lang::En),
        other => bail!("no test speech in {other}"),
    }
}

/// The lead-in moves the chunk seams onto other words.
pub(super) fn record_speech(
    world: &mut E2eWorld,
    lang: Lang,
    condition: Condition,
    lead_in_secs: f32,
) -> Result<()> {
    let samples = synthesize(lang, condition, lead_in_secs)?;
    let path = world.workdir()?.join("speech.wav");
    write_wav(&path, &samples)?;
    world.recording = Some(path);
    world.spoken = Some(lang);
    Ok(())
}
