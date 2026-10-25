//! Steps that check the launched copy: every engine from the bundle, a transcription, a healthy command spotting service and Metal in the engine's log.

use super::{
    bail, ensure, healthy, launch, load_manifest, off_thread, program, read_wav, then, url,
    AudioBuffer16k, BundleLaunch, Context, E2eWorld, OpenAiTranscriber, Path, PathBuf, Result,
    Transcriber, TranscriptionCfg, BACKEND_LOADED, CORPUS, ENGINES, FROM_BUNDLE, METAL_FAILURES,
    METAL_PREFIX, READY_WITHIN, WHISPER_METAL,
};

pub(super) fn all_ready(launch: &BundleLaunch) -> Result<()> {
    let late: Vec<&str> = launch
        .ready
        .iter()
        .filter(|(_, ready)| !ready)
        .map(|(name, _)| name.as_str())
        .collect();
    ensure!(
        late.is_empty() && !launch.ready.is_empty(),
        "not ready within {READY_WITHIN:?}: {late:?}; the app's log:\n{}",
        launch.app_log()
    );
    Ok(())
}

#[then("its log names each engine as taken from the bundle")]
pub(super) fn engines_from_bundle(world: &mut E2eWorld) -> Result<()> {
    let launch = launch(world);
    all_ready(launch)?;
    let log = launch.app_log();
    for engine in ENGINES {
        let program = program(engine)?;
        ensure!(
            log.lines().any(
                |line| line.contains(&format!("runs {program} ")) && line.contains(FROM_BUNDLE)
            ),
            "the app's log does not name {program} {FROM_BUNDLE}:\n{log}"
        );
    }
    Ok(())
}

#[then("a recording is transcribed through the started services")]
pub(super) async fn transcribe(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let ports = launch(world).ports;
        let corpus = PathBuf::from(std::env::var(CORPUS)?);
        let recording = load_manifest(&corpus.join("manifest.jsonl"))?
            .into_iter()
            .next()
            .context("the corpus is empty")?;
        let mut transcriber = OpenAiTranscriber::connect(TranscriptionCfg {
            base_url: url(ports.stt),
            ..TranscriptionCfg::default()
        })?;
        let transcript = transcriber
            .transcribe(&AudioBuffer16k::new(read_wav(&recording.path)?))?
            .context("the bundled speech recognition engine heard nothing")?;
        ensure!(
            !transcript.as_str().trim().is_empty(),
            "an empty transcript of {}",
            recording.path.display()
        );
        Ok(())
    })
    .await
}

#[then("the command spotting service answers its health check")]
pub(super) fn spotting_healthy(world: &mut E2eWorld) -> Result<()> {
    let port = launch(world).ports.kws;
    ensure!(
        healthy(port),
        "the command spotting service on port {port} does not answer its health check"
    );
    Ok(())
}

#[then(
    "the speech recognition engine's log shows it loaded its compute backends from the copy and set up the Metal GPU"
)]
pub(super) fn whisper_on_metal(world: &mut E2eWorld) -> Result<()> {
    let launch = launch(world);
    let copy = launch
        .folder
        .as_ref()
        .context("no copy")?
        .path()
        .to_path_buf();
    let copy = copy.canonicalize().unwrap_or(copy);
    let whisper = program("whisper")?;
    let log = launch.engine_log(&whisper, launch.ports.stt)?;
    let loaded: Vec<&str> = log
        .lines()
        .filter(|line| line.contains(BACKEND_LOADED))
        .collect();
    ensure!(
        !loaded.is_empty(),
        "{whisper} logged no backend it loaded:\n{log}"
    );
    if let Some(line) = loaded
        .iter()
        .find(|line| !loaded_from(line).starts_with(&copy))
    {
        bail!("{whisper} loaded a backend from outside the copy: {line}");
    }
    ensure!(
        log.contains(WHISPER_METAL),
        "{whisper}'s log does not show the Metal GPU ({WHISPER_METAL}):\n{log}"
    );
    if let Some(failure) = log.lines().find(|line| is_metal_failure(line)) {
        bail!("{whisper} failed on Metal: {failure}");
    }
    Ok(())
}

/// The folder a backend-loaded line names, resolved.
fn loaded_from(line: &str) -> PathBuf {
    let from = line.rsplit(' ').next().unwrap_or_default();
    Path::new(from)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(from))
}

/// Whether a log line tells of a failure on Metal.
fn is_metal_failure(line: &str) -> bool {
    line.contains(METAL_PREFIX) && METAL_FAILURES.iter().any(|word| line.contains(word))
}
