use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher_e2e_test::corpus::{load_manifest, select_balanced};
use voicepuncher_e2e_test::wer::wer;

use crate::E2eWorld;

pub const STARTUP_FAILURE_EXIT_CODE: i32 = 2;
/// An empty config: every setting at its default, never the user's own config file.
pub const NO_CONFIG: &str = "/dev/null";
const MAX_MEAN_WER: f64 = 0.30;

#[given(expr = "a recording of silence lasting {int} seconds")]
fn silence_recording(world: &mut E2eWorld, seconds: u32) -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(format!("silence_{seconds}s.wav"));
    write_silence_wav(&path, seconds)?;
    world.recording = Some(path);
    world.workdir = Some(dir);
    Ok(())
}

#[given("the benchmark corpus of Russian and English recordings with reference transcripts")]
fn benchmark_corpus(world: &mut E2eWorld) -> Result<()> {
    let dir = env("VOICEPUNCHER_BENCH_CORPUS")?;
    let mut samples = load_manifest(&Path::new(&dir).join("manifest.jsonl"))?;
    if let Ok(limit) = std::env::var("VOICEPUNCHER_BENCH_LIMIT") {
        let limit = limit
            .parse()
            .context("VOICEPUNCHER_BENCH_LIMIT must be a number")?;
        samples = select_balanced(samples, limit);
    }
    for lang in ["ru", "en"] {
        if !samples.iter().any(|s| s.lang == lang) {
            bail!("benchmark corpus at {dir} has no '{lang}' recordings");
        }
    }
    world.corpus = samples;
    Ok(())
}

#[when(regex = r"^the user transcribes the recording file (with|without) refinement$")]
fn transcribe_recording(world: &mut E2eWorld, refinement: String) -> Result<()> {
    let recording = world.recording.as_deref().context("no recording")?;
    let output = transcribe(recording, refinement == "with")?;
    world.exit_status = Some(output.status);
    Ok(())
}

#[when("the user transcribes every recording of the corpus with refinement")]
fn transcribe_corpus(world: &mut E2eWorld) -> Result<()> {
    eprintln!("\n  {:<20} {:<4} {:>7} {:>8}", "id", "lang", "dur", "wer");
    for sample in &world.corpus {
        let output = transcribe(&sample.path, true)?;
        let hypothesis = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let rate = wer(&sample.reference, &hypothesis);
        eprintln!(
            "  {:<20} {:<4} {:>7.1} {:>8.3}",
            sample.id, sample.lang, sample.duration_s, rate
        );
        world.word_error_rates.push(rate);
    }
    Ok(())
}

#[then("the app does not exit with a startup failure")]
fn no_startup_failure(world: &mut E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if status.code() == Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!("the app exited with a startup failure ({status})");
    }
    Ok(())
}

#[then("the mean word error rate is below 30 %")]
fn mean_wer_below_bar(world: &mut E2eWorld) -> Result<()> {
    let rates = &world.word_error_rates;
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    eprintln!("  mean WER {mean:.3} over {} recordings\n", rates.len());
    if mean >= MAX_MEAN_WER {
        bail!("mean WER {mean:.3} is not below {MAX_MEAN_WER}");
    }
    Ok(())
}

fn transcribe(recording: &Path, with_refinement: bool) -> Result<Output> {
    let mut cmd = Command::new(sut_bin());
    cmd.args(["--config", NO_CONFIG, "--input-file"])
        .arg(recording)
        .arg("--transcription-url")
        .arg(env("VOICEPUNCHER_TRANSCRIPTION_URL")?);
    if with_refinement {
        cmd.arg("--llm-model").arg(env("VOICEPUNCHER_LLM_MODEL")?);
    } else {
        cmd.arg("--no-llm");
    }
    cmd.output()
        .with_context(|| format!("failed to spawn {:?}", cmd.get_program()))
}

pub fn sut_bin() -> PathBuf {
    std::env::var("VOICEPUNCHER_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/voicepuncher")
        })
}

pub fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is not set — run it through `just`"))
}

pub fn write_silence_wav(path: &Path, seconds: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for _ in 0..(16_000 * seconds) {
        writer.write_sample(0i16)?;
    }
    writer.finalize()?;
    Ok(())
}
