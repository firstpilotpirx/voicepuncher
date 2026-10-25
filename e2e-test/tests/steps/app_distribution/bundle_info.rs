//! Steps of the bundle's Info.plist: identity, version, Dock and microphone reason.

use super::{
    crate_version, ensure, given, info_value, stage_bundle, then, when, Context, E2eWorld, Result,
    APP_NAME, BUNDLE_IDENTIFIER, ICON_FILE, ICON_RESOURCE, MICROPHONE_DESCRIPTION, MICROPHONE_KEY,
};

#[given("the bundle the bundle build made")]
pub(super) fn bundle_built(world: &mut E2eWorld) -> Result<()> {
    world.distribution.bundle = Some(stage_bundle()?);
    Ok(())
}

#[when("its information file is read")]
pub(super) fn read_info(world: &mut E2eWorld) -> Result<()> {
    let bundle = world.distribution.bundle.clone().context("no bundle")?;
    for key in [
        "CFBundleIdentifier",
        "CFBundleName",
        "CFBundleDisplayName",
        "CFBundleExecutable",
        "CFBundleIconFile",
        "CFBundleShortVersionString",
        "CFBundleVersion",
        "LSUIElement",
        MICROPHONE_KEY,
    ] {
        let value = info_value(&bundle, key)?
            .with_context(|| format!("the information file lacks {key}"))?;
        world
            .distribution
            .signed
            .info
            .push((key.to_string(), value));
    }
    Ok(())
}

pub(super) fn info<'a>(world: &'a E2eWorld, key: &str) -> &'a str {
    world
        .distribution
        .signed
        .info
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or_default()
}

#[then("it names the bundle identifier, the display name and the icon file the bundle holds")]
pub(super) fn names_identity(world: &mut E2eWorld) -> Result<()> {
    for (key, expected) in [
        ("CFBundleIdentifier", BUNDLE_IDENTIFIER),
        ("CFBundleName", APP_NAME),
        ("CFBundleDisplayName", APP_NAME),
        ("CFBundleExecutable", APP_NAME),
        ("CFBundleIconFile", ICON_FILE),
    ] {
        ensure!(
            info(world, key) == expected,
            "{key} is '{}', expected '{expected}'",
            info(world, key)
        );
    }
    let bundle = world.distribution.bundle.as_ref().context("no bundle")?;
    ensure!(
        bundle.join(ICON_RESOURCE).is_file(),
        "the bundle holds no {ICON_RESOURCE}"
    );
    ensure!(
        bundle.join("Contents/MacOS").join(APP_NAME).is_file(),
        "the bundle holds no executable"
    );
    Ok(())
}

#[then("it carries the crate's version as both the short and the build version")]
pub(super) fn carries_version(world: &mut E2eWorld) -> Result<()> {
    let version = crate_version()?;
    for key in ["CFBundleShortVersionString", "CFBundleVersion"] {
        ensure!(
            info(world, key) == version,
            "{key} is '{}', expected the crate's '{version}'",
            info(world, key)
        );
    }
    Ok(())
}

#[then("it keeps the application out of the Dock")]
pub(super) fn out_of_dock(world: &mut E2eWorld) -> Result<()> {
    ensure!(
        info(world, "LSUIElement") == "true",
        "LSUIElement is '{}', expected true",
        info(world, "LSUIElement")
    );
    Ok(())
}

#[then("it says why the application needs the microphone")]
pub(super) fn microphone_reason(world: &mut E2eWorld) -> Result<()> {
    ensure!(
        info(world, MICROPHONE_KEY) == MICROPHONE_DESCRIPTION,
        "{MICROPHONE_KEY} is '{}'",
        info(world, MICROPHONE_KEY)
    );
    Ok(())
}
