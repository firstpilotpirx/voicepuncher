use std::path::PathBuf;

use owo_colors::{OwoColorize, Stream};

use voicepuncher_e2e_test::corpus::load_manifest;
use voicepuncher_e2e_test::wer::{cer, wer};

#[test]
fn should_report_wer_per_sample() {
    // Given — gate on all required env vars
    let bin_path = match resolve_bin() {
        Some(p) => p,
        None => {
            eprintln!("GATED: binary not found, skipping accuracy test");
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
    let corpus_dir = match std::env::var("VOICEPUNCHER_BENCH_CORPUS") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!("GATED: VOICEPUNCHER_BENCH_CORPUS not set, skipping");
            return;
        }
    };

    let manifest_path = corpus_dir.join("manifest.jsonl");
    let samples = load_manifest(&manifest_path).expect("load manifest");

    if samples.is_empty() {
        eprintln!("WARNING: manifest is empty, skipping accuracy assertions");
        return;
    }

    struct SampleResult {
        id: String,
        lang: String,
        duration_s: f32,
        wer_raw: f64,
        #[allow(dead_code)]
        cer_raw: f64,
        wer_llm: f64,
        cer_llm: f64,
    }

    // When — run each sample in both modes
    let mut results: Vec<SampleResult> = Vec::with_capacity(samples.len());

    for sample in &samples {
        let path = sample.path.to_str().unwrap_or("");

        let raw_output = std::process::Command::new(&bin_path)
            .args([
                "--config",
                "/dev/null",
                "--input-file",
                path,
                "--whisper-model",
                &whisper_model,
                "--no-llm",
            ])
            .output()
            .expect("failed to spawn voicepuncher binary (no-llm)");

        let llm_output = std::process::Command::new(&bin_path)
            .args([
                "--config",
                "/dev/null",
                "--input-file",
                path,
                "--whisper-model",
                &whisper_model,
                "--llm-model",
                &llm_model,
            ])
            .output()
            .expect("failed to spawn voicepuncher binary (with-llm)");

        let hyp_raw = String::from_utf8_lossy(&raw_output.stdout)
            .trim()
            .to_owned();
        let hyp_llm = String::from_utf8_lossy(&llm_output.stdout)
            .trim()
            .to_owned();

        results.push(SampleResult {
            id: sample.id.clone(),
            lang: sample.lang.clone(),
            duration_s: sample.duration_s,
            wer_raw: wer(&sample.reference, &hyp_raw),
            cer_raw: cer(&sample.reference, &hyp_raw),
            wer_llm: wer(&sample.reference, &hyp_llm),
            cer_llm: cer(&sample.reference, &hyp_llm),
        });
    }

    // Then — print comparison table
    const SEP: usize = 74;

    eprintln!();
    eprintln!(
        "  {}",
        format!(
            "WER comparison — raw Whisper vs LLM post-processed  ({} samples)",
            results.len()
        )
        .if_supports_color(Stream::Stderr, |t| t.bold())
    );
    eprintln!("  {}", "─".repeat(SEP));
    eprintln!(
        "  {} {} {} {} {} {} {}",
        format!("{:<20}", "id").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:<4}", "lang").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>7}", "dur").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>8}", "wer_raw").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>8}", "wer_llm").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>7}", "Δwer").if_supports_color(Stream::Stderr, |t| t.dimmed()),
        format!("{:>8}", "cer_llm").if_supports_color(Stream::Stderr, |t| t.dimmed()),
    );
    eprintln!("  {}", "─".repeat(SEP));

    for r in &results {
        let delta = r.wer_llm - r.wer_raw;
        let delta_padded = format!("{delta:>+7.3}");
        let delta_display = if delta < -0.01 {
            format!(
                "{}",
                delta_padded.if_supports_color(Stream::Stderr, |t| t.green())
            )
        } else if delta > 0.01 {
            format!(
                "{}",
                delta_padded.if_supports_color(Stream::Stderr, |t| t.red())
            )
        } else {
            format!(
                "{}",
                delta_padded.if_supports_color(Stream::Stderr, |t| t.dimmed())
            )
        };

        eprintln!(
            "  {:<20} {:<4} {:>7.1} {:>8.3} {:>8.3} {} {:>8.3}",
            &r.id, &r.lang, r.duration_s, r.wer_raw, r.wer_llm, delta_display, r.cer_llm,
        );
    }

    eprintln!("  {}", "─".repeat(SEP));

    let n = results.len() as f64;
    let mean_wer_raw = results.iter().map(|r| r.wer_raw).sum::<f64>() / n;
    let mean_wer_llm = results.iter().map(|r| r.wer_llm).sum::<f64>() / n;
    let mean_cer_llm = results.iter().map(|r| r.cer_llm).sum::<f64>() / n;
    let mean_delta = mean_wer_llm - mean_wer_raw;

    let mean_delta_padded = format!("{mean_delta:>+7.3}");
    let mean_delta_display = if mean_delta < -0.01 {
        format!(
            "{}",
            mean_delta_padded.if_supports_color(Stream::Stderr, |t| t.green())
        )
    } else if mean_delta > 0.01 {
        format!(
            "{}",
            mean_delta_padded.if_supports_color(Stream::Stderr, |t| t.red())
        )
    } else {
        format!(
            "{}",
            mean_delta_padded.if_supports_color(Stream::Stderr, |t| t.dimmed())
        )
    };

    let mean_label = format!("{:<20}", "MEAN");
    let mean_label_display = format!(
        "{}",
        mean_label.if_supports_color(Stream::Stderr, |t| t.bold())
    );
    eprintln!(
        "  {mean_label_display} {:<4} {:>7} {mean_wer_raw:>8.3} {mean_wer_llm:>8.3} {mean_delta_display} {mean_cer_llm:>8.3}",
        "",
        "",
    );
    eprintln!("  {}", "─".repeat(SEP));
    eprintln!();

    assert!(
        mean_wer_llm < 0.30,
        "mean WER (with LLM) {mean_wer_llm:.3} exceeds 30% threshold — check model wiring or resampling",
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
