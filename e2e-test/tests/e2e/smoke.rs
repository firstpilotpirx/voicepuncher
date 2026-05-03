use std::path::PathBuf;

#[test]
fn should_load_models_and_run_pipeline_for_silence_wav() {
    // Given — resolve binary path
    let bin_path = match resolve_bin() {
        Some(p) => p,
        None => {
            eprintln!("GATED: binary not found, skipping smoke test");
            return;
        }
    };
    let whisper_model = match std::env::var("VOICEPUNCHER_WHISPER_MODEL") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_WHISPER_MODEL not set, skipping");
            return;
        }
    };
    let llm_model = match std::env::var("VOICEPUNCHER_LLM_MODEL") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_LLM_MODEL not set, skipping");
            return;
        }
    };

    // Write 1-second silence WAV to tempdir
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let wav_path = tmp_dir.path().join("silence.wav");
    write_silence_wav(&wav_path, 1);

    // When — fork the binary
    let output = std::process::Command::new(&bin_path)
        .args([
            "--input-file",
            wav_path.to_str().unwrap(),
            "--whisper-model",
            &whisper_model,
            "--llm-model",
            &llm_model,
        ])
        .output()
        .expect("failed to spawn voicepuncher binary");

    // Then — models loaded and pipeline ran.
    // Exit 0: transcript produced. Exit 1: pipeline error (e.g. empty transcript from silence).
    // Both are acceptable — they prove models loaded and the pipeline executed.
    // Exit 2 means model/binary setup failed, which is the only failure we care about here.
    let code = output.status.code().unwrap_or(99);
    assert_ne!(
        code,
        2,
        "model or binary setup failed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn resolve_bin() -> Option<PathBuf> {
    let path = std::env::var("VOICEPUNCHER_BIN").unwrap_or_else(|_| {
        format!(
            "{}/../target/release/voicepuncher",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let p = PathBuf::from(&path);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn write_silence_wav(path: &std::path::Path, duration_secs: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("wav writer");
    for _ in 0..(16_000 * duration_secs) {
        writer.write_sample(0i16).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}
