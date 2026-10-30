//! `streaming-bench`: streams the user's labelled clips through the streaming recognizer and
//! prints what it heard and how late, so each speed-up is measured on the same clips.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use voicepuncher_command_spotting::bench::{load_dataset, run_clip, Outcome, Pace, Summary};
use voicepuncher_command_spotting::streaming::engine::{StreamingCfg, StreamingEngine};

#[derive(Parser)]
struct Args {
    /// The labelled clips (`dataset.jsonl`).
    #[arg(
        long,
        default_value = "~/.voicepuncher/research/command-bench/dataset.jsonl"
    )]
    dataset: String,
    #[arg(long, env = "VOICEPUNCHER_STREAMING_EN_MODEL_DIR")]
    en_model_dir: PathBuf,
    #[arg(long, env = "VOICEPUNCHER_STREAMING_RU_MODEL_DIR")]
    ru_model_dir: PathBuf,
    #[arg(long, env = "VOICEPUNCHER_KWS_VAD_MODEL")]
    vad_model: PathBuf,
    #[arg(long, default_value_t = 2)]
    threads: i32,
    #[arg(long, default_value_t = 0.3)]
    min_silence_secs: f32,
    /// Hand the audio over as fast as it decodes: accuracy only, no delays.
    #[arg(long)]
    fast: bool,
    /// Only the clips whose id holds this text.
    #[arg(long)]
    only: Option<String>,
    /// Only every n-th clip: a quick comparison.
    #[arg(long, default_value_t = 1)]
    sample: usize,
    /// Print each clip's outcome, not only the wrong ones.
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let dataset = expand_home(&args.dataset)?;
    let engine = StreamingEngine::load(&StreamingCfg {
        en_model_dir: args.en_model_dir.clone(),
        ru_model_dir: args.ru_model_dir.clone(),
        vad_model: args.vad_model.clone(),
        threads: args.threads,
        min_silence_secs: args.min_silence_secs,
        command_history: None,
    })?;
    let outcomes = run_all(&engine, &args, &dataset)?;
    println!("all   {}", Summary::of(&outcomes));
    for word in ["start", "stop"] {
        let said: Vec<Outcome> = outcomes
            .iter()
            .filter(|o| o.label == [word])
            .cloned()
            .collect();
        println!("{word:<5} {}", Summary::of(&said));
    }
    Ok(())
}

fn run_all(engine: &StreamingEngine, args: &Args, dataset: &Path) -> Result<Vec<Outcome>> {
    let pace = if args.fast {
        Pace::Fast
    } else {
        Pace::RealTime
    };
    let wanted = |id: &str| args.only.as_deref().is_none_or(|o| id.contains(o));
    let mut outcomes = Vec::new();
    let clips = load_dataset(dataset)?
        .into_iter()
        .step_by(args.sample.max(1));
    for clip in clips.filter(|c| wanted(&c.id)) {
        let outcome = run_clip(engine, &clip, pace)?;
        if args.verbose || !outcome.right() {
            print_outcome(&outcome);
        }
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

fn print_outcome(o: &Outcome) {
    let delay = o
        .delay_ms
        .map_or_else(String::new, |d| format!(" {d:.0}ms"));
    let mark = if o.right() { "ok" } else { "✗" };
    println!(
        "{mark} {} said {:?} heard {:?}{delay}",
        o.id, o.label, o.heard
    );
}

fn expand_home(path: &str) -> Result<PathBuf> {
    match path.strip_prefix("~/") {
        Some(rest) => Ok(PathBuf::from(std::env::var("HOME").context("HOME")?).join(rest)),
        None => Ok(PathBuf::from(path)),
    }
}
