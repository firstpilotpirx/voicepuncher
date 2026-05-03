use std::path::PathBuf;

use voicepuncher_e2e_test::corpus::load_manifest;
use voicepuncher_e2e_test::wer::{cer, wer};

#[test]
fn should_report_wer_per_sample() {
    // Given — gate on all four env vars
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

    // When — run binary per sample
    let mut total_wer = 0.0f64;
    let mut count = 0usize;

    eprintln!(
        "{:<20} {:<5} {:>10} {:>8} {:>8}",
        "id", "lang", "duration_s", "wer", "cer"
    );
    for sample in &samples {
        let output = std::process::Command::new(&bin_path)
            .args([
                "--config",
                "/dev/null",
                "--input-file",
                sample.path.to_str().unwrap_or(""),
                "--whisper-model",
                &whisper_model,
                "--llm-model",
                &llm_model,
            ])
            .output()
            .expect("failed to spawn voicepuncher binary");

        let hypothesis = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let w = wer(&sample.reference, &hypothesis);
        let c = cer(&sample.reference, &hypothesis);
        eprintln!(
            "{:<20} {:<5} {:>10.1} {:>8.3} {:>8.3}",
            &sample.id, &sample.lang, sample.duration_s, w, c
        );

        total_wer += w;
        count += 1;
    }

    // Then — assert mean WER < 30%
    let mean_wer = total_wer / count as f64;
    eprintln!("Mean WER over {} samples: {:.3}", count, mean_wer);
    assert!(
        mean_wer < 0.30,
        "mean WER {:.3} exceeds 30% threshold — check model wiring or resampling",
        mean_wer
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
