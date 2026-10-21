use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher_e2e_test::concurrency::map_concurrently;
use voicepuncher_e2e_test::corpus::{load_manifest, select_balanced};
use voicepuncher_e2e_test::wer::wer;

use crate::steps::off_thread;
use crate::E2eWorld;

pub const STARTUP_FAILURE_EXIT_CODE: i32 = 2;
/// An empty config: every setting at its default, never the user's own config file.
pub const NO_CONFIG: &str = "/dev/null";
const MAX_MEAN_WER: f64 = 0.30;
/// Two recordings at a time: speech recognition of one overlaps refinement of the
/// other, while each service still handles one request at a time.
const CONCURRENT_RECORDINGS: usize = 2;

#[given(expr = "a recording of silence lasting {int} seconds")]
fn silence_recording(world: &mut E2eWorld, seconds: u32) -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(format!("silence_{seconds}s.wav"));
    write_silence_wav(&path, seconds)?;
    world.recording = Some(path);
    world.workdir = Some(dir);
    Ok(())
}

#[given(
    expr = "a fixed sample of {int} recordings of the benchmark corpus, half Russian and half English, with reference transcripts"
)]
fn benchmark_sample(world: &mut E2eWorld, size: usize) -> Result<()> {
    let dir = env("VOICEPUNCHER_BENCH_CORPUS")?;
    let samples = load_manifest(&Path::new(&dir).join("manifest.jsonl"))?;
    let mut size = size;
    if let Ok(limit) = std::env::var("VOICEPUNCHER_BENCH_LIMIT") {
        let limit: usize = limit
            .parse()
            .context("VOICEPUNCHER_BENCH_LIMIT must be a number")?;
        size = size.min(limit);
    }
    let samples = select_balanced(samples, size);
    for lang in ["ru", "en"] {
        let count = samples.iter().filter(|s| s.lang == lang).count();
        if count * 2 != size {
            bail!(
                "benchmark corpus at {dir} gives {count} '{lang}' recordings, need {} of {size}",
                size / 2
            );
        }
    }
    world.corpus = samples;
    Ok(())
}

#[when(regex = r"^the user transcribes the recording file (with|without) refinement$")]
async fn transcribe_recording(world: &mut E2eWorld, refinement: String) -> Result<()> {
    off_thread(world, move |world| {
        let recording = world.recording.as_deref().context("no recording")?;
        let output = transcribe(recording, world.config.as_deref(), refinement == "with")?;
        world.exit_status = Some(output.status);
        world.stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Ok(())
    })
    .await
}

#[when("the user transcribes every recording of the sample with refinement")]
async fn transcribe_sample(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let outputs = map_concurrently(&world.corpus, CONCURRENT_RECORDINGS, |sample| {
            transcribe(&sample.path, None, true)
        });
        eprintln!("\n  {:<20} {:<4} {:>7} {:>8}", "id", "lang", "dur", "wer");
        for (sample, output) in world.corpus.iter().zip(outputs) {
            let hypothesis = String::from_utf8_lossy(&output?.stdout).trim().to_owned();
            let rate = wer(&sample.reference, &hypothesis);
            eprintln!(
                "  {:<20} {:<4} {:>7.1} {:>8.3}",
                sample.id, sample.lang, sample.duration_s, rate
            );
            world.word_error_rates.push(rate);
        }
        Ok(())
    })
    .await
}

#[then("the app does not exit with a startup failure")]
fn no_startup_failure(world: &mut E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if status.code() == Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!("the app exited with a startup failure ({status})");
    }
    Ok(())
}

#[then("no permission is checked")]
fn no_permission_checked(world: &mut E2eWorld) -> Result<()> {
    if let Some(line) = world.stderr.lines().find(|l| l.contains("permissions:")) {
        bail!("file mode checked the permissions: {line}");
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

pub fn transcribe(
    recording: &Path,
    config: Option<&Path>,
    with_refinement: bool,
) -> Result<Output> {
    let mut cmd = transcribe_command(recording, config, with_refinement)?;
    cmd.output()
        .with_context(|| format!("failed to spawn {:?}", cmd.get_program()))
}

/// The app's command line that transcribes `recording` through the local services.
pub fn transcribe_command(
    recording: &Path,
    config: Option<&Path>,
    with_refinement: bool,
) -> Result<Command> {
    let mut cmd = Command::new(sut_bin());
    cmd.arg("--config")
        .arg(config.unwrap_or(Path::new(NO_CONFIG)))
        .arg("--input-file")
        .arg(recording)
        .arg("--transcription-url")
        .arg(env("VOICEPUNCHER_TRANSCRIPTION_URL")?);
    if with_refinement {
        cmd.arg("--llm")
            .arg("--refinement-url")
            .arg(env("VOICEPUNCHER_REFINEMENT_URL")?);
    } else {
        cmd.arg("--no-llm");
    }
    Ok(cmd)
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
