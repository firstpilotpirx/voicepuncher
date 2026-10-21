//! Steps of `app_distribution.feature`: the bundle and the disk image the bundle stage of
//! `full-check` just built in `dist/`, and the justfile recipes that build, sign and guard them.
//! Every build a scenario makes itself goes to the scenario's own folder (`dist_dir`), so the
//! stage's signed bundle is never replaced.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use crate::steps::off_thread;
use crate::E2eWorld;

const APP_NAME: &str = "voicepuncher";
const BUNDLE_DIR: &str = "voicepuncher.app";
const BUNDLE_IDENTIFIER: &str = "io.github.firstpilotpirx.voicepuncher";
const ICON_FILE: &str = "voicepuncher";
const ICON_RESOURCE: &str = "Contents/Resources/voicepuncher.icns";
const INFO_FILE: &str = "Contents/Info.plist";
const MICROPHONE_KEY: &str = "NSMicrophoneUsageDescription";
const MICROPHONE_DESCRIPTION: &str =
    "voicepuncher records your voice while you hold the hotkey, to turn it into text on this Mac.";
const IDENTITY_VARIABLE: &str = "VOICEPUNCHER_SIGNING_IDENTITY";
/// The command `_signing-identity-steps` names, which creates the certificate.
const CREATE_CERTIFICATE: &str = "just signing-identity";
/// What the bundle build prints once it has built the bundle; a refusal never prints it.
const BUILT_LINE: &str = "✓ bundle";
/// The first dependency of `release`: the release runs nothing before it. A test runs this guard,
/// never `release` itself, so a broken guard cannot publish anything.
const RELEASE_GUARD: &str = "_require-release-identity";
/// The bundle stage's own check of a bundle and its disk image.
const BUNDLE_CHECK: &str = "_check-bundle";
const BACKGROUND_PICTURE: &str = ".background/dmg-background.png";
/// The Finder's layout of the window: icon positions, size and background.
const WINDOW_LAYOUT: &str = ".DS_Store";

/// What a scenario built, ran and read.
#[derive(Debug, Default)]
pub struct DistributionRun {
    /// The value the scenario gives the signing identity: `Some("")` for none configured.
    identity: Option<String>,
    bundle: Option<PathBuf>,
    bundles: Vec<PathBuf>,
    mount: Option<Mounted>,
    info: Vec<(String, String)>,
    requirements: Vec<String>,
    verified: Option<Output>,
    run: Option<Output>,
    /// The folder a scenario's bundle build writes to.
    dist: Option<PathBuf>,
    stages: Vec<String>,
}

/// A disk image attached read-only at a folder of the scenario, detached when dropped.
#[derive(Debug)]
struct Mounted(PathBuf);

impl Drop for Mounted {
    fn drop(&mut self) {
        let _ = Command::new("hdiutil")
            .args(["detach", "-quiet"])
            .arg(&self.0)
            .status();
    }
}

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the E2E crate lives in the checkout")
        .to_path_buf()
}

fn stage_dist() -> PathBuf {
    checkout().join("dist")
}

/// The crate's version, as `app_version` in the justfile reads it.
fn crate_version() -> Result<String> {
    let manifest = fs::read_to_string(checkout().join("app/Cargo.toml"))?;
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .map(str::to_string)
        .context("app/Cargo.toml names no version")
}

/// The variables cargo sets for a test process. A build script reads some of them (ring reruns when
/// `CARGO_MANIFEST_DIR` changes), so a release build that inherited them would rebuild the whole
/// crate graph, and the next build without them would rebuild it again.
fn is_test_variable(name: &str) -> bool {
    [
        "CARGO_MANIFEST_",
        "CARGO_PKG_",
        "CARGO_CRATE_",
        "CARGO_BIN_",
        "CARGO_PRIMARY_PACKAGE",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
        || ["CARGO_TARGET_TMPDIR", "CARGO_RUSTC_CURRENT_DIR", "OUT_DIR"].contains(&name)
}

fn just(args: &[&str], identity: Option<&str>) -> Result<Output> {
    let mut command = Command::new("just");
    command
        .arg("--justfile")
        .arg(checkout().join("justfile"))
        .args(args)
        .current_dir(checkout());
    for (name, _) in std::env::vars_os() {
        if name.to_str().is_some_and(is_test_variable) {
            command.env_remove(name);
        }
    }
    if let Some(identity) = identity {
        command.env(IDENTITY_VARIABLE, identity);
    }
    command.output().context("just could not start")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_tool(program: &str, args: &[&str]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("{program} could not start"))
}

fn info_value(bundle: &Path, key: &str) -> Result<Option<String>> {
    let plist = bundle.join(INFO_FILE);
    let output = run_tool(
        "plutil",
        &["-extract", key, "raw", "-o", "-", path_str(&plist)?],
    )?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str().context("a path that is not UTF-8")
}

fn copy_bundle(from: &Path, to: &Path) -> Result<()> {
    let status = Command::new("cp").arg("-R").arg(from).arg(to).status()?;
    ensure!(status.success(), "copying {} failed", from.display());
    Ok(())
}

fn stage_bundle() -> Result<PathBuf> {
    let bundle = stage_dist().join(BUNDLE_DIR);
    ensure!(
        bundle.is_dir(),
        "{} is missing: the bundle stage builds it first",
        bundle.display()
    );
    Ok(bundle)
}

fn stage_disk_image() -> Result<PathBuf> {
    let image = stage_dist().join(format!("{APP_NAME}-v{}.dmg", crate_version()?));
    ensure!(
        image.is_file(),
        "{} is missing: the bundle stage builds it first",
        image.display()
    );
    Ok(image)
}

/// The stages a recipe hands `_stages`, in order (`"$stage"` is the accuracy stage `full-check`
/// picks).
fn recipe_stages(recipe: &str) -> Result<Vec<String>> {
    let output = just(&["--show", recipe], None)?;
    ensure!(output.status.success(), "{}", text(&output));
    let shown = String::from_utf8_lossy(&output.stdout).to_string();
    let line = shown
        .lines()
        .find_map(|line| {
            line.split_once("_stages ")
                .map(|(_, rest)| rest.to_string())
        })
        .with_context(|| format!("{recipe} runs no _stages:\n{shown}"))?;
    Ok(line.split_whitespace().map(str::to_string).collect())
}

/// A stage `_stages` runs under the machine-wide lock, by its rule: an E2E stage, the refinement
/// evaluation, and every stage after the first of them.
fn waits_for_the_lock(stage: &str) -> bool {
    stage.starts_with("e2e") || stage == "refine-eval" || stage == "\"$stage\""
}

#[given("the disk image the release build made")]
fn release_disk_image(world: &mut E2eWorld) -> Result<()> {
    world.distribution.bundles = vec![stage_disk_image()?];
    Ok(())
}

#[when("the user opens it")]
async fn open_disk_image(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let image = world.distribution.bundles[0].clone();
        let mount = world.workdir()?.join("volume");
        fs::create_dir_all(&mount)?;
        let output = run_tool(
            "hdiutil",
            &[
                "attach",
                "-nobrowse",
                "-readonly",
                "-mountpoint",
                path_str(&mount)?,
                path_str(&image)?,
            ],
        )?;
        ensure!(
            output.status.success(),
            "the disk image did not open: {}",
            text(&output)
        );
        world.distribution.mount = Some(Mounted(mount));
        Ok(())
    })
    .await
}

fn mounted(world: &E2eWorld) -> Result<&Path> {
    Ok(&world
        .distribution
        .mount
        .as_ref()
        .context("the disk image is not open")?
        .0)
}

#[then("it holds the application and a shortcut to the Applications folder")]
fn holds_app_and_shortcut(world: &mut E2eWorld) -> Result<()> {
    let volume = mounted(world)?;
    ensure!(
        volume.join(BUNDLE_DIR).join(INFO_FILE).is_file(),
        "the volume holds no application bundle"
    );
    let shortcut = fs::read_link(volume.join("Applications"))
        .context("the volume holds no Applications shortcut")?;
    ensure!(
        shortcut == Path::new("/Applications"),
        "the Applications shortcut points to {}",
        shortcut.display()
    );
    let mut entries: Vec<String> = fs::read_dir(volume)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| !name.starts_with('.'))
        .collect();
    entries.sort();
    ensure!(
        entries == ["Applications", BUNDLE_DIR],
        "the volume shows {entries:?}, expected the application and the Applications shortcut only"
    );
    Ok(())
}

#[then("its window is laid out over the picture that says to drag the application to Applications")]
fn window_over_background(world: &mut E2eWorld) -> Result<()> {
    let volume = mounted(world)?;
    let picture = fs::read(volume.join(BACKGROUND_PICTURE))
        .context("the volume holds no background picture")?;
    let packaged = fs::read(checkout().join("app/packaging/dmg-background.png"))?;
    ensure!(
        picture == packaged,
        "the background picture is not the packaged one"
    );
    ensure!(
        volume.join(WINDOW_LAYOUT).is_file(),
        "the volume holds no window layout"
    );
    Ok(())
}

#[then("its volume is named after the application and the version")]
fn volume_named(world: &mut E2eWorld) -> Result<()> {
    let volume = mounted(world)?;
    let info = Command::new("diskutil")
        .args(["info", "-plist"])
        .arg(volume)
        .output()?;
    ensure!(info.status.success(), "{}", text(&info));
    let mut extract = Command::new("plutil")
        .args(["-extract", "VolumeName", "raw", "-o", "-", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    std::io::Write::write_all(
        extract.stdin.as_mut().context("plutil's input")?,
        &info.stdout,
    )?;
    let name = extract.wait_with_output()?;
    let name = String::from_utf8_lossy(&name.stdout).trim().to_string();
    let expected = format!("{APP_NAME} {}", crate_version()?);
    ensure!(
        name == expected,
        "the volume is named '{name}', expected '{expected}'"
    );
    Ok(())
}

#[given("the bundle the bundle build made")]
fn bundle_built(world: &mut E2eWorld) -> Result<()> {
    world.distribution.bundle = Some(stage_bundle()?);
    Ok(())
}

#[when("its information file is read")]
fn read_info(world: &mut E2eWorld) -> Result<()> {
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
        world.distribution.info.push((key.to_string(), value));
    }
    Ok(())
}

fn info<'a>(world: &'a E2eWorld, key: &str) -> &'a str {
    world
        .distribution
        .info
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or_default()
}

#[then("it names the bundle identifier, the display name and the icon file the bundle holds")]
fn names_identity(world: &mut E2eWorld) -> Result<()> {
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
fn carries_version(world: &mut E2eWorld) -> Result<()> {
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
fn out_of_dock(world: &mut E2eWorld) -> Result<()> {
    ensure!(
        info(world, "LSUIElement") == "true",
        "LSUIElement is '{}', expected true",
        info(world, "LSUIElement")
    );
    Ok(())
}

#[then("it says why the application needs the microphone")]
fn microphone_reason(world: &mut E2eWorld) -> Result<()> {
    ensure!(
        info(world, MICROPHONE_KEY) == MICROPHONE_DESCRIPTION,
        "{MICROPHONE_KEY} is '{}'",
        info(world, MICROPHONE_KEY)
    );
    Ok(())
}

#[given("two different builds signed with the same signing identity")]
async fn two_builds(world: &mut E2eWorld) -> Result<()> {
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
            fs::create_dir_all(folder.join(name))?;
            let bundle = folder.join(name).join(BUNDLE_DIR);
            copy_bundle(&built, &bundle)?;
            let plist = bundle.join(INFO_FILE);
            let replaced = run_tool(
                "plutil",
                &[
                    "-replace",
                    "CFBundleVersion",
                    "-string",
                    &build_version,
                    path_str(&plist)?,
                ],
            )?;
            ensure!(replaced.status.success(), "{}", text(&replaced));
            let signed = just(&["_sign", path_str(&bundle)?], None)?;
            ensure!(signed.status.success(), "signing failed: {}", text(&signed));
            world.distribution.bundles.push(bundle);
        }
        Ok(())
    })
    .await
}

#[when("their designated requirements are read")]
fn read_requirements(world: &mut E2eWorld) -> Result<()> {
    for bundle in world.distribution.bundles.clone() {
        let output = run_tool("codesign", &["-d", "-r-", path_str(&bundle)?])?;
        let shown = text(&output);
        let requirement = shown
            .lines()
            .find_map(|line| line.strip_prefix("designated => "))
            .with_context(|| format!("no designated requirement: {shown}"))?;
        world
            .distribution
            .requirements
            .push(requirement.to_string());
    }
    Ok(())
}

/// The SHA-1 of the certificate the signing identity names, as the keychain lists it.
fn certificate_hash() -> Result<String> {
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
fn requirements_name_certificate(world: &mut E2eWorld) -> Result<()> {
    let leaf = format!("certificate leaf = H\"{}\"", certificate_hash()?);
    ensure!(
        world.distribution.requirements.len() == 2,
        "two builds expected"
    );
    for requirement in &world.distribution.requirements {
        ensure!(
            requirement.contains(&leaf),
            "'{requirement}' does not name the certificate ({leaf})"
        );
    }
    Ok(())
}

#[then("they are identical")]
fn requirements_identical(world: &mut E2eWorld) -> Result<()> {
    let requirements = &world.distribution.requirements;
    ensure!(
        requirements[0] == requirements[1],
        "the requirements differ: '{}' and '{}'",
        requirements[0],
        requirements[1]
    );
    Ok(())
}

#[when("its signature is verified strictly, every nested file included")]
fn verify_strictly(world: &mut E2eWorld) -> Result<()> {
    let bundle = world.distribution.bundle.clone().context("no bundle")?;
    world.distribution.verified = Some(run_tool(
        "codesign",
        &["--verify", "--deep", "--strict", path_str(&bundle)?],
    )?);
    Ok(())
}

#[then("the verification passes")]
fn verification_passes(world: &mut E2eWorld) -> Result<()> {
    let verified = world
        .distribution
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

#[given("no signing identity is configured")]
fn no_identity(world: &mut E2eWorld) {
    world.distribution.identity = Some(String::new());
}

#[given("the configured signing identity is not in the keychain")]
fn missing_identity(world: &mut E2eWorld) {
    world.distribution.identity = Some(format!("voicepuncher-missing-{}", std::process::id()));
}

#[when("the release is run")]
async fn run_release(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let identity = world.distribution.identity.clone();
        world.distribution.run = Some(just(&[RELEASE_GUARD], identity.as_deref())?);
        Ok(())
    })
    .await
}

#[when("the bundle build is run")]
async fn run_bundle_build(world: &mut E2eWorld) -> Result<()> {
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

fn run_output(world: &E2eWorld) -> Result<&Output> {
    world.distribution.run.as_ref().context("nothing was run")
}

#[then("it stops before building anything")]
fn stops_before_building(world: &mut E2eWorld) -> Result<()> {
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
fn names_create_command(world: &mut E2eWorld) -> Result<()> {
    let shown = text(run_output(world)?);
    ensure!(
        shown.contains(CREATE_CERTIFICATE),
        "it does not name '{CREATE_CERTIFICATE}': {shown}"
    );
    Ok(())
}

#[then("it names the identity and the steps to create the certificate")]
fn names_identity_and_steps(world: &mut E2eWorld) -> Result<()> {
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
fn produces_verified_bundle(world: &mut E2eWorld) -> Result<()> {
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
fn says_ad_hoc(world: &mut E2eWorld) -> Result<()> {
    let shown = text(run_output(world)?);
    ensure!(
        shown.contains("ad-hoc") && shown.contains("no stable identity"),
        "it does not say the bundle is signed ad-hoc without a stable identity: {shown}"
    );
    Ok(())
}

#[given("a checkout with the bundle tools installed")]
fn bundle_tools(_world: &mut E2eWorld) -> Result<()> {
    let output = just(&["_require-create-dmg"], None)?;
    ensure!(output.status.success(), "{}", text(&output));
    Ok(())
}

#[when("the stages of the full check are listed")]
fn list_full_check(world: &mut E2eWorld) -> Result<()> {
    world.distribution.stages = recipe_stages("full-check")?;
    Ok(())
}

#[when("the stages of the quick check are listed")]
fn list_quick_check(world: &mut E2eWorld) -> Result<()> {
    world.distribution.stages = recipe_stages("check")?;
    Ok(())
}

#[then("a bundle stage comes before the stages that wait for the machine-wide lock")]
fn bundle_before_locked(world: &mut E2eWorld) -> Result<()> {
    let stages = &world.distribution.stages;
    let bundle = stages
        .iter()
        .position(|stage| stage == "bundle")
        .with_context(|| format!("no bundle stage in {stages:?}"))?;
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
fn no_bundle_stage(world: &mut E2eWorld) -> Result<()> {
    let stages = &world.distribution.stages;
    ensure!(
        !stages.is_empty() && !stages.iter().any(|stage| stage == "bundle"),
        "the quick check lists {stages:?}"
    );
    Ok(())
}

#[given("a bundle whose information file lacks the microphone usage description")]
async fn broken_bundle(world: &mut E2eWorld) -> Result<()> {
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
async fn stage_checks(world: &mut E2eWorld) -> Result<()> {
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
fn fails_naming_key(world: &mut E2eWorld) -> Result<()> {
    let run = run_output(world)?;
    let shown = text(run);
    ensure!(!run.status.success(), "the check passed: {shown}");
    ensure!(
        shown.contains(MICROPHONE_KEY),
        "the failure does not name {MICROPHONE_KEY}: {shown}"
    );
    Ok(())
}
