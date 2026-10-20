//! The packaging sources of the application bundle: the Info.plist template and the icon set.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const INFO_PLIST: &str = "packaging/Info.plist";
const ICONSET: &str = "packaging/icons/voicepuncher.iconset";
const ICONSET_FILES: [&str; 10] = [
    "icon_16x16.png",
    "icon_16x16@2x.png",
    "icon_32x32.png",
    "icon_32x32@2x.png",
    "icon_128x128.png",
    "icon_128x128@2x.png",
    "icon_256x256.png",
    "icon_256x256@2x.png",
    "icon_512x512.png",
    "icon_512x512@2x.png",
];

fn packaging_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn info_plist_as_json() -> Value {
    let output = Command::new("plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(packaging_path(INFO_PLIST))
        .output()
        .expect("plutil runs");
    assert!(
        output.status.success(),
        "Info.plist does not lint: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("plutil prints JSON")
}

#[test]
fn should_carry_the_keys_that_identify_the_application() {
    // Given
    let plist = info_plist_as_json();
    let expected: [(&str, Value); 8] = [
        (
            "CFBundleIdentifier",
            "io.github.firstpilotpirx.voicepuncher".into(),
        ),
        ("CFBundleName", "voicepuncher".into()),
        ("CFBundleDisplayName", "voicepuncher".into()),
        ("CFBundleExecutable", "voicepuncher".into()),
        ("CFBundleIconFile", "voicepuncher".into()),
        ("LSMinimumSystemVersion", "15.0".into()),
        (
            "LSApplicationCategoryType",
            "public.app-category.productivity".into(),
        ),
        ("LSUIElement", Value::Bool(true)),
    ];

    // Then
    for (key, value) in expected {
        assert_eq!(plist.get(key), Some(&value), "{key}");
    }
}

#[test]
fn should_explain_why_the_microphone_is_needed() {
    // Given
    let plist = info_plist_as_json();

    // Then
    let description = plist
        .get("NSMicrophoneUsageDescription")
        .and_then(Value::as_str)
        .expect("NSMicrophoneUsageDescription is a string");
    assert_eq!(
        description,
        "voicepuncher records your voice while you hold the hotkey, to turn it into text on this Mac."
    );
}

#[test]
fn should_leave_the_version_keys_to_the_bundle_build() {
    // Given
    let plist = info_plist_as_json();

    // Then: the recipe sets both from Cargo.toml, so the template holds no stale version
    assert_eq!(plist.get("CFBundleShortVersionString"), None);
    assert_eq!(plist.get("CFBundleVersion"), None);
}

#[test]
fn should_hold_every_size_of_the_icon_set() {
    // Given
    let iconset = packaging_path(ICONSET);

    // Then
    let missing: Vec<&str> = ICONSET_FILES
        .iter()
        .copied()
        .filter(|file| !iconset.join(file).is_file())
        .collect();
    assert!(
        missing.is_empty(),
        "missing in {}: {missing:?}",
        iconset.display()
    );
}
