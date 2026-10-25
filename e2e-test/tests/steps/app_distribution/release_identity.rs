//! Steps of the release and bundle build without or with a signing identity.

use super::{
    ensure, given, just, off_thread, path_str, run_output, run_tool, text, then, when, Context,
    E2eWorld, Result, BUILT_LINE, BUNDLE_DIR, CREATE_CERTIFICATE, IDENTITY_VARIABLE, RELEASE_GUARD,
};

#[given("no signing identity is configured")]
pub(super) fn no_identity(world: &mut E2eWorld) {
    world.distribution.identity = Some(String::new());
}

#[given("the configured signing identity is not in the keychain")]
pub(super) fn missing_identity(world: &mut E2eWorld) {
    world.distribution.identity = Some(format!("voicepuncher-missing-{}", std::process::id()));
}

#[when("the release is run")]
pub(super) async fn run_release(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let identity = world.distribution.identity.clone();
        world.distribution.run = Some(just(&[RELEASE_GUARD], identity.as_deref())?);
        Ok(())
    })
    .await
}

#[when("the bundle build is run")]
pub(super) async fn run_bundle_build(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let dist = world.workdir()?.join("dist");
        let identity = world.distribution.identity.clone();
        world.distribution.run = Some(just(
            &["--set", "dist_dir", path_str(&dist)?, "bundle"],
            identity.as_deref(),
        )?);
        world.distribution.dist = Some(dist);
        Ok(())
    })
    .await
}

#[then("it stops before building anything")]
pub(super) fn stops_before_building(world: &mut E2eWorld) -> Result<()> {
    let run = run_output(world)?;
    let shown = text(run);
    ensure!(!run.status.success(), "it went on: {shown}");
    ensure!(
        !shown.contains(BUILT_LINE) && !shown.contains("Compiling"),
        "it built before stopping: {shown}"
    );
    if let Some(dist) = &world.distribution.dist {
        ensure!(
            !dist.join(BUNDLE_DIR).exists(),
            "it left a bundle in {}",
            dist.display()
        );
    }
    Ok(())
}

#[then("it names the command that creates the certificate")]
pub(super) fn names_create_command(world: &mut E2eWorld) -> Result<()> {
    let shown = text(run_output(world)?);
    ensure!(
        shown.contains(CREATE_CERTIFICATE),
        "it does not name '{CREATE_CERTIFICATE}': {shown}"
    );
    Ok(())
}

#[then("it names the identity and the steps to create the certificate")]
pub(super) fn names_identity_and_steps(world: &mut E2eWorld) -> Result<()> {
    let shown = text(run_output(world)?);
    let identity = world.distribution.identity.as_deref().unwrap_or_default();
    ensure!(
        shown.contains(identity),
        "it does not name '{identity}': {shown}"
    );
    ensure!(
        shown.contains(CREATE_CERTIFICATE) && shown.contains(IDENTITY_VARIABLE),
        "it does not give the steps to create the certificate: {shown}"
    );
    Ok(())
}

#[then("it produces a bundle whose signature verifies")]
pub(super) fn produces_verified_bundle(world: &mut E2eWorld) -> Result<()> {
    let run = run_output(world)?;
    ensure!(
        run.status.success(),
        "the bundle build failed: {}",
        text(run)
    );
    let bundle = world
        .distribution
        .dist
        .as_ref()
        .context("no folder")?
        .join(BUNDLE_DIR);
    let verified = run_tool(
        "codesign",
        &["--verify", "--deep", "--strict", path_str(&bundle)?],
    )?;
    ensure!(
        verified.status.success(),
        "the signature does not verify: {}",
        text(&verified)
    );
    Ok(())
}

#[then("it says the bundle is signed ad-hoc and has no stable identity")]
pub(super) fn says_ad_hoc(world: &mut E2eWorld) -> Result<()> {
    let shown = text(run_output(world)?);
    ensure!(
        shown.contains("ad-hoc") && shown.contains("no stable identity"),
        "it does not say the bundle is signed ad-hoc without a stable identity: {shown}"
    );
    Ok(())
}
