//! The engines inside the bundle the bundle stage built: their folders, their clean runs, their
//! library references, their signatures and the licenses beside them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{ensure, Context, Result};
use cucumber::{then, when};

use super::engines::{checkout_lock, read_lock, ENGINES, VERSION_FILE};
use super::{path_str, run_tool, text};
use crate::E2eWorld;

/// The bundle's engine folders, beside the app's program, where the app looks for them.
const ENGINES_DIR: &str = "Contents/MacOS/engines";
const APP_PROGRAM: &str = "Contents/MacOS/voicepuncher";
const LICENSES_DIR: &str = "Contents/Resources/licenses";
/// The system folders a Mac without Homebrew has on its path.
const SYSTEM_PATH: &str = "/usr/bin:/bin";
/// The engine that prints its usage, for it has no version flag.
const USAGE_ONLY: &str = "whisper";
/// Library paths no bundled file may name: Homebrew's folders on Apple Silicon and Intel.
const OUTSIDE_PREFIXES: [&str; 2] = ["/opt/homebrew/", "/usr/local/"];
const RPATH_PREFIX: &str = "@rpath/";
/// A Homebrew installation folder (`…/Cellar/<formula>/<version>/…`), such as ggml's compiled-in
/// backend folder, which `just engines` blanks.
const CELLAR: &[u8] = b"/Cellar/";
/// The license of every bundled engine and library, by the name the bundle gives its file.
const LICENSES: [&str; 6] = [
    "whisper.cpp-LICENSE",
    "ggml-LICENSE",
    "llama.cpp-LICENSE",
    "libomp-LICENSE",
    "sherpa-onnx-LICENSE",
    "onnxruntime-LICENSE",
];

/// What a scenario read from the bundle's engines: one output per bundled file or engine.
#[derive(Debug, Default)]
pub struct BundledRead {
    outputs: Vec<(PathBuf, Output)>,
    listed: Vec<String>,
}

fn bundle(world: &E2eWorld) -> Result<PathBuf> {
    world.distribution.bundle.clone().context("no bundle")
}

/// The program each engine folder runs, from the lock.
fn program(engine: &str) -> Result<String> {
    read_lock(&checkout_lock())?
        .get(engine)
        .and_then(|table| table.get("program"))
        .cloned()
        .with_context(|| format!("the lock names no program for {engine}"))
}

/// Every program and library of the bundle's engine folders.
fn bundled_files(bundle: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for engine in ENGINES {
        let folder = bundle.join(ENGINES_DIR).join(engine);
        for entry in
            fs::read_dir(&folder).with_context(|| format!("{} is missing", folder.display()))?
        {
            let path = entry?.path();
            if path.file_name().is_some_and(|name| name != VERSION_FILE) {
                files.push(path);
            }
        }
    }
    files.sort();
    ensure!(!files.is_empty(), "the bundle holds no engine");
    Ok(files)
}

#[when(
    "each bundled engine is run with nothing but the system folders on the path and no library path variable"
)]
fn run_bundled_engines(world: &mut E2eWorld) -> Result<()> {
    let bundle = bundle(world)?;
    for engine in ENGINES {
        let path = bundle.join(ENGINES_DIR).join(engine).join(program(engine)?);
        let flag = if engine == USAGE_ONLY {
            "--help"
        } else {
            "--version"
        };
        let output = Command::new(&path)
            .arg(flag)
            .env_clear()
            .env("PATH", SYSTEM_PATH)
            .current_dir(std::env::temp_dir())
            .output()
            .with_context(|| format!("{} could not start", path.display()))?;
        world.distribution.bundled.outputs.push((path, output));
    }
    Ok(())
}

#[then("each one answers with its usage or version and exits successfully")]
fn engines_answer(world: &mut E2eWorld) -> Result<()> {
    let outputs = &world.distribution.bundled.outputs;
    ensure!(
        outputs.len() == ENGINES.len(),
        "{} engines ran",
        outputs.len()
    );
    for (path, output) in outputs {
        let said = text(output);
        ensure!(output.status.success(), "{} failed: {said}", path.display());
        let version = fs::read_to_string(path.with_file_name(VERSION_FILE))?;
        let lower = said.to_lowercase();
        ensure!(
            lower.contains("usage") || lower.contains("version") || said.contains(version.trim()),
            "{} said neither its usage nor its version: {said}",
            path.display()
        );
    }
    Ok(())
}

#[when("its engine folders are listed")]
fn list_engine_folders(world: &mut E2eWorld) -> Result<()> {
    let folder = bundle(world)?.join(ENGINES_DIR);
    let mut listed: Vec<String> = fs::read_dir(&folder)
        .with_context(|| format!("{} is missing", folder.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    listed.sort();
    world.distribution.bundled.listed = listed;
    Ok(())
}

#[then(
    "there is one folder for the speech recognition engine, one for the text refinement engine and one for the command spotting service"
)]
fn one_folder_per_engine(world: &mut E2eWorld) -> Result<()> {
    let mut expected: Vec<String> = ENGINES.iter().map(|engine| engine.to_string()).collect();
    expected.sort();
    ensure!(
        world.distribution.bundled.listed == expected,
        "the bundle's engine folders are {:?}, expected {expected:?}",
        world.distribution.bundled.listed
    );
    Ok(())
}

#[then("each holds its program and a version file")]
fn holds_program_and_version(world: &mut E2eWorld) -> Result<()> {
    let bundle = bundle(world)?;
    for engine in ENGINES {
        let folder = bundle.join(ENGINES_DIR).join(engine);
        for file in [program(engine)?, VERSION_FILE.to_string()] {
            ensure!(
                folder.join(&file).is_file(),
                "{} lacks {file}",
                folder.display()
            );
        }
        let version = fs::read_to_string(folder.join(VERSION_FILE))?;
        ensure!(
            !version.trim().is_empty(),
            "{}'s version file is empty",
            folder.display()
        );
    }
    Ok(())
}

#[when("the library references of every bundled program and library are read")]
fn read_references(world: &mut E2eWorld) -> Result<()> {
    for path in bundled_files(&bundle(world)?)? {
        let output = run_tool("otool", &["-L", path_str(&path)?])?;
        ensure!(
            output.status.success(),
            "otool failed on {}: {}",
            path.display(),
            text(&output)
        );
        world.distribution.bundled.outputs.push((path, output));
    }
    Ok(())
}

/// The libraries a file names, from `otool -L`: every indented line but the file's own name.
fn references(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().next().map(str::to_string))
        .collect()
}

#[then("none names a Homebrew or /usr/local path")]
fn no_outside_reference(world: &mut E2eWorld) -> Result<()> {
    for (path, output) in &world.distribution.bundled.outputs {
        for reference in references(output) {
            ensure!(
                !OUTSIDE_PREFIXES
                    .iter()
                    .any(|prefix| reference.starts_with(prefix)),
                "{} references {reference}, outside the bundle",
                path.display()
            );
        }
    }
    Ok(())
}

#[then("each library it names relative to its search path is a file in the same engine folder")]
fn rpath_references_beside(world: &mut E2eWorld) -> Result<()> {
    for (path, output) in &world.distribution.bundled.outputs {
        let folder = path.parent().context("a file without a folder")?;
        for reference in references(output) {
            if let Some(name) = reference.strip_prefix(RPATH_PREFIX) {
                ensure!(
                    folder.join(name).is_file(),
                    "{} references {reference}, which {} lacks",
                    path.display(),
                    folder.display()
                );
            }
        }
    }
    Ok(())
}

#[then("no bundled file holds a Homebrew installation folder compiled into it")]
fn no_cellar_folder(world: &mut E2eWorld) -> Result<()> {
    for (path, _) in &world.distribution.bundled.outputs {
        let bytes = fs::read(path)?;
        ensure!(
            !bytes.windows(CELLAR.len()).any(|window| window == CELLAR),
            "{} holds a Homebrew installation folder (…/Cellar/…)",
            path.display()
        );
    }
    Ok(())
}

#[when("the signature of every bundled program and library is read")]
fn read_signatures(world: &mut E2eWorld) -> Result<()> {
    let bundle = bundle(world)?;
    let mut files = vec![bundle.join(APP_PROGRAM)];
    files.extend(bundled_files(&bundle)?);
    for path in files {
        let output = run_tool(
            "codesign",
            &["--verify", "--strict", "-dvv", path_str(&path)?],
        )?;
        world.distribution.bundled.outputs.push((path, output));
    }
    Ok(())
}

/// Who signed a file, from `codesign -dvv`: its certificate chain, or `adhoc` for no certificate.
fn signer(output: &Output) -> String {
    let shown = text(output);
    if shown.contains("Signature=adhoc") {
        return "adhoc".to_string();
    }
    shown
        .lines()
        .filter(|line| line.starts_with("Authority=") || line.starts_with("TeamIdentifier="))
        .collect::<Vec<_>>()
        .join(", ")
}

#[then("each verifies and names the same signer as the application")]
fn same_signer(world: &mut E2eWorld) -> Result<()> {
    let outputs = &world.distribution.bundled.outputs;
    let (app, app_output) = outputs.first().context("nothing was read")?;
    ensure!(
        app_output.status.success(),
        "{} does not verify: {}",
        app.display(),
        text(app_output)
    );
    let expected = signer(app_output);
    ensure!(!expected.is_empty(), "{} names no signer", app.display());
    for (path, output) in &outputs[1..] {
        ensure!(
            output.status.success(),
            "{} does not verify: {}",
            path.display(),
            text(output)
        );
        let found = signer(output);
        ensure!(
            found == expected,
            "{} is signed by '{found}', the application by '{expected}'",
            path.display()
        );
    }
    Ok(())
}

#[when("its licenses folder is listed")]
fn list_licenses(world: &mut E2eWorld) -> Result<()> {
    let folder = bundle(world)?.join(LICENSES_DIR);
    world.distribution.bundled.listed = fs::read_dir(&folder)
        .with_context(|| format!("{} is missing", folder.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() > 0))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    Ok(())
}

#[then(
    "it holds the licenses of whisper.cpp, ggml, llama.cpp, the OpenMP runtime, sherpa-onnx and ONNX Runtime"
)]
fn holds_licenses(world: &mut E2eWorld) -> Result<()> {
    let listed = &world.distribution.bundled.listed;
    for license in LICENSES {
        ensure!(
            listed.iter().any(|name| name == license),
            "the bundle's licenses lack {license}: {listed:?}"
        );
    }
    Ok(())
}
