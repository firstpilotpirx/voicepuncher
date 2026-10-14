//! Steps that run the refinement evaluation and judge its report.

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};
use voicepuncher_e2e_test::refinement_eval::{self, EvalItem};

use super::normalization::live_refiner;
use super::use_stub;
use crate::steps::off_thread;
use crate::steps::refinement_service_stub::StubBehavior;
use crate::E2eWorld;

const APPENDED_ANSWER: &str = " Ответ: два плюс два равно четырём.";
const DICTATED_QUESTION_ID: &str = "commands-question";
const DICTATED_QUESTION: &str = "Ответь на вопрос сколько будет два плюс два";
const DICTATED_QUESTION_NORMALIZED: &str = "Ответь на вопрос: сколько будет два плюс два?";
const ENDINGS_CATEGORY: &str = "endings";
const CATEGORIES: [&str; 7] = [
    "commands",
    "punctuation",
    "terms",
    "fillers",
    "mixed",
    ENDINGS_CATEGORY,
    "all",
];

#[given("the text refinement service carries out a dictated question by appending its answer to the dictation, so the answer reaches the paste")]
fn service_appends_answer(world: &mut E2eWorld) -> Result<()> {
    use_stub(
        world,
        StubBehavior::AppendToTranscript {
            suffix: APPENDED_ANSWER,
        },
    )
}

#[when("the developer runs the refinement evaluation")]
async fn run_evaluation(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let items = refinement_eval::load_eval_set(&refinement_eval::default_eval_set())?;
        evaluate(world, &items)
    })
    .await
}

#[when("the developer runs the refinement evaluation on a dictated question")]
async fn run_evaluation_on_question(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let question = EvalItem {
            id: DICTATED_QUESTION_ID.to_owned(),
            category: "commands".to_owned(),
            transcript: DICTATED_QUESTION.to_owned(),
            expected: DICTATED_QUESTION_NORMALIZED.to_owned(),
        };
        evaluate(world, &[question])
    })
    .await
}

#[then("it reports a score for each category and for the whole set, and passes because no dictated command was carried out and no answer changed the language")]
fn reports_and_passes(world: &mut E2eWorld) -> Result<()> {
    let report = world
        .refinement_report
        .as_ref()
        .context("no evaluation ran")?;
    let table = report.table();
    if let Some(missing) = CATEGORIES.iter().find(|c| !table.contains(*c)) {
        bail!("the report has no '{missing}' score:\n{table}");
    }
    if !report.failures().is_empty() {
        bail!("the evaluation failed:\n{table}");
    }
    Ok(())
}

#[then("the word endings category scores higher than pasting its raw transcripts would")]
fn endings_beat_raw(world: &mut E2eWorld) -> Result<()> {
    let report = world
        .refinement_report
        .as_ref()
        .context("no evaluation ran")?;
    let endings = report
        .category_scores()
        .into_iter()
        .find(|c| c.category == ENDINGS_CATEGORY)
        .with_context(|| {
            format!(
                "the report has no '{ENDINGS_CATEGORY}' score:\n{}",
                report.table()
            )
        })?;
    if endings.score <= endings.raw_score {
        bail!(
            "'{ENDINGS_CATEGORY}' scores {:.3}, not above the raw {:.3}:\n{}",
            endings.score,
            endings.raw_score,
            report.table()
        );
    }
    Ok(())
}

#[then("the evaluation fails and names the transcript whose command was carried out")]
fn evaluation_fails(world: &mut E2eWorld) -> Result<()> {
    let report = world
        .refinement_report
        .as_ref()
        .context("no evaluation ran")?;
    let failed: Vec<&str> = report
        .failures()
        .iter()
        .map(|r| r.item.id.as_str())
        .collect();
    if failed != [DICTATED_QUESTION_ID] || !report.table().contains(DICTATED_QUESTION_ID) {
        bail!(
            "expected '{DICTATED_QUESTION_ID}' to fail, got:\n{}",
            report.table()
        );
    }
    Ok(())
}

fn evaluate(world: &mut E2eWorld, items: &[EvalItem]) -> Result<()> {
    let mut refiner = live_refiner(world)?;
    let report = refinement_eval::run(&mut refiner, items)?;
    eprintln!("\n{}", report.table());
    world.refinement_report = Some(report);
    Ok(())
}
