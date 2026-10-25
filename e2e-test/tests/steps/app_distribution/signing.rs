//! Steps of the signature: two builds name the same certificate and verify strictly.

use super::{
    copy_bundle, crate_version, ensure, fs, given, just, off_thread, path_str, run_tool,
    stage_bundle, text, then, when, Context, E2eWorld, Result, BUNDLE_DIR, IDENTITY_VARIABLE,
    INFO_FILE,
};

#[given("two different builds signed with the same signing identity")]
pub(super) async fn two_builds(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let built = stage_bundle()?;
        let version = crate_version()?;
        let folder = world.workdir()?.to_path_buf();
        // The second build differs from the first in its build version, so its contents and its
        // signature differ; both are signed with the identity the checkout names
        for (name, build_version) in [
            ("first", version.clone()),
            ("later", format!("{version}.1")),
        ] {
            let bundle = signed_copy(&built, &folder.join(name), &build_version)?;
            world.distribution.bundles.push(bundle);
        }
        Ok(())
    })
    .await
}

/// A copy of the `built` bundle in `folder` with `build_version`, signed with the identity the
/// checkout names.
fn signed_copy(
    built: &std::path::Path,
    folder: &std::path::Path,
    build_version: &str,
) -> Result<std::path::PathBuf> {
    fs::create_dir_all(folder)?;
    let bundle = folder.join(BUNDLE_DIR);
    copy_bundle(built, &bundle)?;
    let plist = bundle.join(INFO_FILE);
    let replaced = run_tool(
        "plutil",
        &[
            "-replace",
            "CFBundleVersion",
            "-string",
            build_version,
            path_str(&plist)?,
        ],
    )?;
    ensure!(replaced.status.success(), "{}", text(&replaced));
    let signed = just(&["_sign", path_str(&bundle)?], None)?;
    ensure!(signed.status.success(), "signing failed: {}", text(&signed));
    Ok(bundle)
}

#[when("their designated requirements are read")]
pub(super) fn read_requirements(world: &mut E2eWorld) -> Result<()> {
    for bundle in world.distribution.bundles.clone() {
        let output = run_tool("codesign", &["-d", "-r-", path_str(&bundle)?])?;
        let shown = text(&output);
        let requirement = shown
            .lines()
            .find_map(|line| line.strip_prefix("designated => "))
            .with_context(|| format!("no designated requirement: {shown}"))?;
        world
            .distribution
            .signed
            .requirements
            .push(requirement.to_string());
    }
    Ok(())
}

/// The SHA-1 of the certificate the signing identity names, as the keychain lists it.
pub(super) fn certificate_hash() -> Result<String> {
    let identity = std::env::var(IDENTITY_VARIABLE)?;
    let output = run_tool("security", &["find-identity", "-v", "-p", "codesigning"])?;
    let listed = String::from_utf8_lossy(&output.stdout).to_string();
    listed
        .lines()
        .find(|line| line.contains(&format!("\"{identity}\"")))
        .and_then(|line| line.split_whitespace().nth(1))
        .map(str::to_lowercase)
        .with_context(|| format!("the keychain lists no identity '{identity}'"))
}

#[then("both name the signing certificate")]
pub(super) fn requirements_name_certificate(world: &mut E2eWorld) -> Result<()> {
    let leaf = format!("certificate leaf = H\"{}\"", certificate_hash()?);
    ensure!(
        world.distribution.signed.requirements.len() == 2,
        "two builds expected"
    );
    for requirement in &world.distribution.signed.requirements {
        ensure!(
            requirement.contains(&leaf),
            "'{requirement}' does not name the certificate ({leaf})"
        );
    }
    Ok(())
}

#[then("they are identical")]
pub(super) fn requirements_identical(world: &mut E2eWorld) -> Result<()> {
    let requirements = &world.distribution.signed.requirements;
    ensure!(
        requirements[0] == requirements[1],
        "the requirements differ: '{}' and '{}'",
        requirements[0],
        requirements[1]
    );
    Ok(())
}

#[when("its signature is verified strictly, every nested file included")]
pub(super) fn verify_strictly(world: &mut E2eWorld) -> Result<()> {
    let bundle = world.distribution.bundle.clone().context("no bundle")?;
    world.distribution.signed.verified = Some(run_tool(
        "codesign",
        &["--verify", "--deep", "--strict", path_str(&bundle)?],
    )?);
    Ok(())
}

#[then("the verification passes")]
pub(super) fn verification_passes(world: &mut E2eWorld) -> Result<()> {
    let verified = world
        .distribution
        .signed
        .verified
        .as_ref()
        .context("not verified")?;
    ensure!(
        verified.status.success(),
        "the signature does not verify: {}",
        text(verified)
    );
    Ok(())
}
