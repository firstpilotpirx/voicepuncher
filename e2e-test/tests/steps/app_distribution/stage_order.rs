//! Steps of the check stages around the bundle: its place before the locked stages and its engine stage.

use super::{
    copy_bundle, ensure, given, just, off_thread, path_str, recipe_stages, run_output, run_tool,
    stage_bundle, stage_disk_image, text, then, waits_for_the_lock, when, Context, E2eWorld,
    Result, BUNDLE_CHECK, BUNDLE_DIR, INFO_FILE, MICROPHONE_KEY,
};

#[given("a checkout with the bundle tools installed")]
pub(super) fn bundle_tools(_world: &mut E2eWorld) -> Result<()> {
    let output = just(&["_require-create-dmg"], None)?;
    ensure!(output.status.success(), "{}", text(&output));
    Ok(())
}

#[when("the stages of the full check are listed")]
pub(super) fn list_full_check(world: &mut E2eWorld) -> Result<()> {
    world.distribution.stages = recipe_stages("full-check")?;
    Ok(())
}

/// The model stage of `full-check` that runs the bundle's own engines.
pub(super) const BUNDLE_ENGINE_STAGE: &str = "e2e-bundle";

#[then(
    "a bundle engine stage comes after the bundle stage, among the stages that wait for the machine-wide lock"
)]
pub(super) fn bundle_engines_locked(world: &mut E2eWorld) -> Result<()> {
    let stages = &world.distribution.stages;
    let bundle = position_of(stages, "bundle")?;
    let engines = position_of(stages, BUNDLE_ENGINE_STAGE)?;
    ensure!(
        engines > bundle,
        "{BUNDLE_ENGINE_STAGE} runs before the bundle it needs: {stages:?}"
    );
    ensure!(
        waits_for_the_lock(BUNDLE_ENGINE_STAGE),
        "{BUNDLE_ENGINE_STAGE} does not wait for the machine-wide lock"
    );
    Ok(())
}

#[when("the stages of the quick check are listed")]
pub(super) fn list_quick_check(world: &mut E2eWorld) -> Result<()> {
    world.distribution.stages = recipe_stages("check")?;
    Ok(())
}

#[then("a bundle stage comes before the stages that wait for the machine-wide lock")]
pub(super) fn bundle_before_locked(world: &mut E2eWorld) -> Result<()> {
    let stages = &world.distribution.stages;
    let bundle = position_of(stages, "bundle")?;
    ensure!(
        !stages[..bundle]
            .iter()
            .any(|stage| waits_for_the_lock(stage)),
        "the bundle stage comes after a stage that waits for the lock: {stages:?}"
    );
    ensure!(
        stages[bundle + 1..]
            .iter()
            .any(|stage| waits_for_the_lock(stage)),
        "no stage waits for the lock after the bundle stage: {stages:?}"
    );
    Ok(())
}

#[then("there is no bundle stage")]
pub(super) fn no_bundle_stage(world: &mut E2eWorld) -> Result<()> {
    let stages = &world.distribution.stages;
    ensure!(
        !stages.is_empty() && !stages.iter().any(|stage| stage == "bundle"),
        "the quick check lists {stages:?}"
    );
    Ok(())
}

#[given("a bundle whose information file lacks the microphone usage description")]
pub(super) async fn broken_bundle(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let bundle = world.workdir()?.join(BUNDLE_DIR);
        copy_bundle(&stage_bundle()?, &bundle)?;
        let plist = bundle.join(INFO_FILE);
        let removed = run_tool("plutil", &["-remove", MICROPHONE_KEY, path_str(&plist)?])?;
        ensure!(removed.status.success(), "{}", text(&removed));
        world.distribution.bundle = Some(bundle);
        Ok(())
    })
    .await
}

#[when("the full check's bundle stage checks it")]
pub(super) async fn stage_checks(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let bundle = world.distribution.bundle.clone().context("no bundle")?;
        let image = stage_disk_image()?;
        world.distribution.run = Some(just(
            &[BUNDLE_CHECK, path_str(&bundle)?, path_str(&image)?],
            None,
        )?);
        Ok(())
    })
    .await
}

#[then("the check fails naming the missing key")]
pub(super) fn fails_naming_key(world: &mut E2eWorld) -> Result<()> {
    let run = run_output(world)?;
    let shown = text(run);
    ensure!(!run.status.success(), "the check passed: {shown}");
    ensure!(
        shown.contains(MICROPHONE_KEY),
        "the failure does not name {MICROPHONE_KEY}: {shown}"
    );
    Ok(())
}

/// Where the stage `name` comes among `stages`.
fn position_of(stages: &[String], name: &str) -> Result<usize> {
    stages
        .iter()
        .position(|stage| stage == name)
        .with_context(|| format!("no {name} stage in {stages:?}"))
}
