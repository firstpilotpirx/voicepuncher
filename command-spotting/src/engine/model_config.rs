//! The sherpa-onnx configuration of the keyword spotter and the voice activity detector, and the model files they load.

use super::{
    anyhow, keywords, path_string, Context, EngineCfg, Keyword, KeywordSpotterConfig, Path, Result,
    SileroVadModelConfig, VadModelConfig, MAX_SPEECH_SECS, MIN_SPEECH_SECS, SAMPLE_RATE,
    SILERO_WINDOW, VAD_SPEECH_THRESHOLD,
};

pub(super) fn spotter_config(
    cfg: &EngineCfg,
    keywords: &[Keyword],
) -> Result<KeywordSpotterConfig> {
    let mut config = KeywordSpotterConfig::default();
    let transducer = &mut config.model_config.transducer;
    transducer.encoder = Some(model_file(&cfg.model_dir, "encoder")?);
    transducer.decoder = Some(model_file(&cfg.model_dir, "decoder")?);
    transducer.joiner = Some(model_file(&cfg.model_dir, "joiner")?);
    config.model_config.tokens = Some(path_string(&cfg.model_dir.join("tokens.txt")));
    config.model_config.num_threads = cfg.threads;
    config.keywords_buf = Some(keywords::all_lines(keywords));
    Ok(config)
}

pub(super) fn vad_config(cfg: &EngineCfg) -> VadModelConfig {
    VadModelConfig {
        silero_vad: SileroVadModelConfig {
            model: Some(path_string(&cfg.vad_model)),
            threshold: VAD_SPEECH_THRESHOLD,
            min_silence_duration: cfg.min_silence_secs,
            min_speech_duration: MIN_SPEECH_SECS,
            window_size: i32::try_from(SILERO_WINDOW).unwrap_or(i32::MAX),
            max_speech_duration: MAX_SPEECH_SECS,
        },
        sample_rate: i32::try_from(SAMPLE_RATE).unwrap_or(i32::MAX),
        num_threads: 1,
        ..VadModelConfig::default()
    }
}

/// The int8 model file whose name starts with `part`.
pub(super) fn model_file(dir: &Path, part: &str) -> Result<String> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("read model directory {}", dir.display()))?;
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name.starts_with(part) && name.ends_with(".int8.onnx")
        })
        .map(|p| path_string(&p))
        .ok_or_else(|| anyhow!("no {part}*.int8.onnx in {}", dir.display()))
}
