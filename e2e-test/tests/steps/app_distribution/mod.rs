//! Steps of `app_distribution.feature`: the bundle and the disk image the bundle stage of
//! `full-check` just built in `dist/`, and the justfile recipes that build, sign and guard them.
//! Every build a scenario makes itself goes to the scenario's own folder (`dist_dir`), so the
//! stage's signed bundle is never replaced.

use std::fs;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use crate::steps::off_thread;
use crate::E2eWorld;

mod bundle_launch;
mod bundled;
mod engines;

mod tools;
use tools::{
    checkout, copy_bundle, crate_version, info_value, just, path_str, recipe_stages, run_output,
    run_tool, stage_bundle, stage_disk_image, text, waits_for_the_lock,
};
mod bundle_info;
mod disk_image;
mod release_identity;
mod signing;
mod stage_order;

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
    signed: SignedRead,
    run: Option<Output>,
    /// The folder a scenario's bundle build writes to.
    dist: Option<PathBuf>,
    stages: Vec<String>,
    engines: engines::EngineRun,
    bundled: bundled::BundledRead,
    launch: bundle_launch::BundleLaunch,
}

/// What a scenario read of a bundle's information and signature.
#[derive(Debug, Default)]
struct SignedRead {
    info: Vec<(String, String)>,
    requirements: Vec<String>,
    verified: Option<Output>,
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
