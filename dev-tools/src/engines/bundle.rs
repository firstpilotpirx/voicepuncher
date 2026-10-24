//! `dev-tools check-bundle <app> <dmg>`: a bundle's Info.plist checked key by key, then its signature verified
//! strictly; its engines each in a folder with its program and `VERSION`, every reference inside that folder,
//! each file signed by the app's signer, each engine running with nothing but the system folders on `$PATH`; the
//! licenses; and its disk image mounted and found to hold the app and the Applications shortcut. The first
//! failure names what is wrong.

use super::relocate::{check_folder, files};
use super::{capture, crate_version, run};
use crate::context::Context;
use crate::Outcome;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const ENGINES: [(&str, &str, &str); 3] = [
    ("whisper", "whisper-server", "--help"),
    ("llama", "llama-server", "--version"),
    (
        "command-spotting",
        "voicepuncher-command-spotting",
        "--version",
    ),
];
const LICENSES: [&str; 6] = [
    "whisper.cpp",
    "ggml",
    "llama.cpp",
    "libomp",
    "sherpa-onnx",
    "onnxruntime",
];

/// The Info.plist keys and values a bundle of this version holds.
pub fn expected_keys(version: &str) -> Vec<(&'static str, String)> {
    let fixed = [
        ("CFBundleIdentifier", "io.github.firstpilotpirx.voicepuncher"),
        ("CFBundleName", "voicepuncher"),
        ("CFBundleDisplayName", "voicepuncher"),
        ("CFBundleExecutable", "voicepuncher"),
        ("CFBundleIconFile", "voicepuncher"),
        ("CFBundleShortVersionString", version),
        ("CFBundleVersion", version),
        ("LSMinimumSystemVersion", "15.0"),
        ("LSUIElement", "true"),
        ("LSApplicationCategoryType", "public.app-category.productivity"),
        ("NSMicrophoneUsageDescription", "voicepuncher records your voice while you hold the hotkey, to turn it into text on this Mac."),
    ];
    fixed
        .iter()
        .map(|(key, value)| (*key, (*value).to_string()))
        .collect()
}

fn info(app: &Path, version: &str) -> Result<(), String> {
    let plist = app.join("Contents/Info.plist");
    let plist = plist.to_string_lossy();
    run("plutil", &["-lint", &plist])?;
    for (key, want) in expected_keys(version) {
        let out = Command::new("plutil")
            .args(["-extract", key, "raw", "-o", "-", &plist])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("Info.plist lacks {key}"));
        }
        let got = String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string();
        if got != want {
            return Err(format!("Info.plist: {key} is '{got}', expected '{want}'"));
        }
    }
    if !app.join("Contents/Resources/voicepuncher.icns").is_file() {
        return Err("bundle lacks Resources/voicepuncher.icns".into());
    }
    run(
        "codesign",
        &["--verify", "--deep", "--strict", &app.to_string_lossy()],
    )
}

/// Who signed a file: its authorities and team, or that it is signed ad-hoc.
fn signer(file: &Path) -> String {
    let out = Command::new("codesign")
        .arg("-dvv")
        .arg(file)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stderr).into_owned())
        .unwrap_or_default();
    out.lines()
        .filter(|line| {
            line.starts_with("Authority=")
                || line.starts_with("TeamIdentifier=")
                || line.contains("Signature=adhoc")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn engine(
    app: &Path,
    (folder, program, flag): (&str, &str, &str),
    app_signer: &str,
) -> Result<(), String> {
    let dir = app.join("Contents/MacOS/engines").join(folder);
    let runs = dir
        .join(program)
        .metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
    if !runs
        || dir
            .join("VERSION")
            .metadata()
            .map_or(true, |meta| meta.len() == 0)
    {
        return Err(format!(
            "bundle lacks engines/{folder}/{program} or its VERSION"
        ));
    }
    check_folder(&dir, &format!("engines/{folder}"))?;
    for file in files(&dir) {
        let shown = file.to_string_lossy();
        if capture("codesign", &["--verify", "--strict", &shown]).is_err() {
            return Err(format!("{shown}: signature does not verify"));
        }
        if signer(&file) != app_signer {
            return Err(format!("{shown} is not signed by the app's signer"));
        }
    }
    let ran = Command::new(dir.join(program))
        .arg(flag)
        .current_dir("/")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output();
    if !ran.is_ok_and(|out| out.status.success()) {
        return Err(format!(
            "engines/{folder}/{program} {flag} fails with nothing but the system folders on $PATH"
        ));
    }
    Ok(())
}

fn disk_image(dmg: &str) -> Result<(), String> {
    let mount = tempfile::tempdir().map_err(|e| e.to_string())?;
    let point = mount.path().to_string_lossy().into_owned();
    capture(
        "hdiutil",
        &[
            "attach",
            "-nobrowse",
            "-readonly",
            "-mountpoint",
            &point,
            dmg,
        ],
    )?;
    let found = if !mount.path().join("voicepuncher.app").is_dir() {
        Err(format!("{dmg} holds no voicepuncher.app"))
    } else if !mount.path().join("Applications").is_symlink() {
        Err(format!("{dmg} holds no Applications shortcut"))
    } else {
        Ok(())
    };
    // A detach that fails leaves a read-only mount macOS drops at the next restart; the result stands
    let _ = capture("hdiutil", &["detach", &point, "-quiet"]);
    found
}

pub fn command(args: &[String]) -> Outcome {
    let [app, dmg] = args else {
        return Err("check-bundle: give the app bundle and its disk image".into());
    };
    let app = std::fs::canonicalize(app).map_err(|e| format!("{app}: {e}"))?;
    let version = crate_version(&Context::from_env().root.join("app/Cargo.toml"));
    info(&app, &version)?;
    let app_signer = signer(&app.join("Contents/MacOS/voicepuncher"));
    for found in ENGINES {
        engine(&app, found, &app_signer)?;
    }
    for license in LICENSES {
        let file = app.join(format!("Contents/Resources/licenses/{license}-LICENSE"));
        if file.metadata().map_or(true, |meta| meta.len() == 0) {
            return Err(format!("bundle lacks the license {license}-LICENSE"));
        }
    }
    disk_image(dmg)?;
    println!(
        "✓ bundle stage: {} signed and verified with its engines and licenses, {dmg} holds the app and the Applications shortcut",
        app.display()
    );
    Ok(0)
}
