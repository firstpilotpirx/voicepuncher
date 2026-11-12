//! The application's own license texts in the bundle's resources.

use std::fs;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{then, when};

use super::checkout;
use crate::E2eWorld;

const RESOURCES_DIR: &str = "Contents/Resources";

#[when("its resources are listed")]
fn list_resources(world: &mut E2eWorld) -> Result<()> {
    let bundle = world.distribution.bundle.clone().context("no bundle")?;
    let folder = bundle.join(RESOURCES_DIR);
    world.distribution.resources = fs::read_dir(&folder)
        .with_context(|| format!("{} is missing", folder.display()))?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    Ok(())
}

#[then(expr = "they hold {string} and {string} equal to the repository's files")]
fn hold_license_texts(world: &mut E2eWorld, first: String, second: String) -> Result<()> {
    let bundle = world.distribution.bundle.clone().context("no bundle")?;
    let listed = &world.distribution.resources;
    for name in [first, second] {
        ensure!(
            listed.contains(&name),
            "the bundle's resources lack {name}: {listed:?}"
        );
        let shipped = fs::read(bundle.join(RESOURCES_DIR).join(&name))?;
        let repository = fs::read(checkout().join(&name))
            .with_context(|| format!("the repository has no {name}"))?;
        ensure!(
            shipped == repository,
            "the bundle's {name} differs from the repository's"
        );
    }
    Ok(())
}
