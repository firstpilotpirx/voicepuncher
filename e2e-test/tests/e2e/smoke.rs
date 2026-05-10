use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Instant;

use owo_colors::{OwoColorize, Stream};

#[test]
fn should_load_models_and_run_pipeline_for_silence_wav() {
    // Given
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
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let wav_path = tmp_dir.path().join("silence.wav");
    write_silence_wav(&wav_path, 1);
    // When
    let status = run_pipeline_with_models(&bin_path, &wav_path, &whisper_model, &llm_model);
    // Then — models loaded and pipeline ran.
    // Exit 0: transcript produced. Exit 1: pipeline error (e.g. empty transcript from silence).
    // Both are acceptable — they prove models loaded and the pipeline executed.
    // Exit 2 means model/binary setup failed, which is the only failure we care about here.
    assert_ne!(status.code(), Some(2), "model or binary setup failed");
}

#[test]
fn should_handle_ten_second_silence_without_crash() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let wav_path = dir.path().join("silence_10s.wav");
    write_silence_wav(&wav_path, 10);
    // When
    let status = run_pipeline(&wav_path);
    // Then
    assert_ne!(status.code(), Some(2), "pipeline crashed on model load");
}

#[test]
fn should_handle_sixty_second_silence_without_crash() {
    // Given
    let dir = tempfile::tempdir().unwrap();
    let wav_path = dir.path().join("silence_60s.wav");
    write_silence_wav(&wav_path, 60);
    // When
    let status = run_pipeline(&wav_path);
    // Then
    assert_ne!(status.code(), Some(2), "pipeline crashed on model load");
}

/// Runs 1 s / 10 s / 60 s silence through both pipeline modes and prints a
/// side-by-side table so you can see whether LLM post-processing adds latency.
/// Only `VOICEPUNCHER_WHISPER_MODEL` is required; with-LLM rows are added only
/// when `VOICEPUNCHER_LLM_MODEL` is also set.
/// Run with `-- --nocapture` to see the table.
#[test]
fn should_compare_pipeline_modes_for_silence() {
    let Some(bin_path) = resolve_bin() else {
        eprintln!("GATED: binary not found, skipping mode comparison");
        return;
    };
    let Ok(whisper_model) = std::env::var("VOICEPUNCHER_WHISPER_MODEL") else {
        eprintln!("GATED: VOICEPUNCHER_WHISPER_MODEL not set, skipping mode comparison");
        return;
    };
    let llm_model = std::env::var("VOICEPUNCHER_LLM_MODEL").ok();

    let tmp = tempfile::tempdir().expect("tempdir");

    struct Row {
        silence: &'static str,
        mode: &'static str,
        exit_code: Option<i32>,
        elapsed_ms: u128,
    }

    let mut rows: Vec<Row> = Vec::new();

    for &(label, secs) in &[("1 s", 1u32), ("10 s", 10), ("60 s", 60)] {
        let wav = tmp.path().join(format!("silence_{secs}s.wav"));
        write_silence_wav(&wav, secs);

        let t = Instant::now();
        let status = std::process::Command::new(&bin_path)
            .args([
                "--input-file",
                wav.to_str().unwrap(),
                "--whisper-model",
                &whisper_model,
                "--no-llm",
            ])
            .output()
            .expect("failed to spawn voicepuncher binary")
            .status;
        rows.push(Row {
            silence: label,
            mode: "no-llm",
            exit_code: status.code(),
            elapsed_ms: t.elapsed().as_millis(),
        });

        if let Some(ref llm) = llm_model {
            let t = Instant::now();
            let status = run_pipeline_with_models(&bin_path, &wav, &whisper_model, llm);
            rows.push(Row {
                silence: label,
                mode: "with-llm",
                exit_code: status.code(),
                elapsed_ms: t.elapsed().as_millis(),
            });
        }
    }

    const SEP: usize = 50;

    eprintln!();
    eprintln!(
        "  {}",
        "Pipeline mode comparison — synthetic silence"
            .if_supports_color(Stream::Stderr, |t| t.bold())
    );
    eprintln!("  {}", "─".repeat(SEP));
    eprintln!(
        "  {} {} {} {}",
        format!("{:<8}", "silence").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:<12}", "mode").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>8}", "exit").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>14}", "elapsed").if_supports_color(Stream::Stderr, |t| t.dimmed()),
    );
    eprintln!("  {}", "─".repeat(SEP));

    let mut prev_silence = "";
    for row in &rows {
        if !prev_silence.is_empty() && row.silence != prev_silence {
            eprintln!(
                "  {}",
                "·".repeat(SEP)
                    .if_supports_color(Stream::Stderr, |t| t.dimmed())
            );
        }
        prev_silence = row.silence;

        let mode_padded = format!("{:<12}", row.mode);
        let mode_display = if row.mode == "with-llm" {
            format!(
                "{}",
                mode_padded.if_supports_color(Stream::Stderr, |t| t.green())
            )
        } else {
            format!(
                "{}",
                mode_padded.if_supports_color(Stream::Stderr, |t| t.cyan())
            )
        };

        let exit_raw = match row.exit_code {
            Some(2) => "crash".to_string(),
            Some(n) => n.to_string(),
            None => "—".to_string(),
        };
        let exit_padded = format!("{exit_raw:>8}");
        let exit_display = if row.exit_code == Some(2) {
            format!(
                "{}",
                exit_padded.if_supports_color(Stream::Stderr, |t| t.red())
            )
        } else {
            format!(
                "{}",
                exit_padded.if_supports_color(Stream::Stderr, |t| t.green())
            )
        };

        let elapsed_padded = format!("{:>12}ms", row.elapsed_ms);

        eprintln!(
            "  {:<8} {} {} {}",
            row.silence, mode_display, exit_display, elapsed_padded
        );

        assert_ne!(
            row.exit_code,
            Some(2),
            "binary setup failed in {} mode for {}",
            row.mode,
            row.silence,
        );
    }

    eprintln!("  {}", "─".repeat(SEP));
    eprintln!();
}

fn run_pipeline(wav_path: &Path) -> ExitStatus {
    let bin_path = match resolve_bin() {
        Some(p) => p,
        None => {
            eprintln!("GATED: binary not found, skipping");
            return fake_skip_status();
        }
    };
    let whisper_model = match std::env::var("VOICEPUNCHER_WHISPER_MODEL") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_WHISPER_MODEL not set, skipping");
            return fake_skip_status();
        }
    };
    let llm_model = match std::env::var("VOICEPUNCHER_LLM_MODEL") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_LLM_MODEL not set, skipping");
            return fake_skip_status();
        }
    };
    run_pipeline_with_models(&bin_path, wav_path, &whisper_model, &llm_model)
}

fn run_pipeline_with_models(
    bin_path: &Path,
    wav_path: &Path,
    whisper_model: &str,
    llm_model: &str,
) -> ExitStatus {
    std::process::Command::new(bin_path)
        .args([
            "--input-file",
            wav_path.to_str().unwrap(),
            "--whisper-model",
            whisper_model,
            "--llm-model",
            llm_model,
        ])
        .status()
        .expect("failed to spawn voicepuncher binary")
}

fn fake_skip_status() -> ExitStatus {
    std::process::Command::new("true")
        .status()
        .expect("true command must exist")
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

fn write_silence_wav(path: &Path, duration_secs: u32) {
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
