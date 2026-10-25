//! The checkout, the just recipes, the tools and the staged bundle the distribution steps run.

use super::{
    ensure, fs, Command, Context, E2eWorld, Output, Path, PathBuf, Result, APP_NAME, BUNDLE_DIR,
    IDENTITY_VARIABLE, INFO_FILE,
};

pub(super) fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the E2E crate lives in the checkout")
        .to_path_buf()
}

pub(super) fn stage_dist() -> PathBuf {
    checkout().join("dist")
}

/// The crate's version, as `app_version` in the justfile reads it.
pub(super) fn crate_version() -> Result<String> {
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
pub(super) fn is_test_variable(name: &str) -> bool {
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

pub(super) fn just(args: &[&str], identity: Option<&str>) -> Result<Output> {
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

pub(super) fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub(super) fn run_tool(program: &str, args: &[&str]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("{program} could not start"))
}

pub(super) fn info_value(bundle: &Path, key: &str) -> Result<Option<String>> {
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

pub(super) fn path_str(path: &Path) -> Result<&str> {
    path.to_str().context("a path that is not UTF-8")
}

pub(super) fn copy_bundle(from: &Path, to: &Path) -> Result<()> {
    let status = Command::new("cp").arg("-R").arg(from).arg(to).status()?;
    ensure!(status.success(), "copying {} failed", from.display());
    Ok(())
}

pub(super) fn stage_bundle() -> Result<PathBuf> {
    let bundle = stage_dist().join(BUNDLE_DIR);
    ensure!(
        bundle.is_dir(),
        "{} is missing: the bundle stage builds it first",
        bundle.display()
    );
    Ok(bundle)
}

pub(super) fn stage_disk_image() -> Result<PathBuf> {
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
pub(super) fn recipe_stages(recipe: &str) -> Result<Vec<String>> {
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
    let mut stages = Vec::new();
    let mut rest = line.as_str();
    while let Some((literal, after)) = rest.split_once("{{") {
        stages.extend(literal.split_whitespace().map(str::to_string));
        let (expression, tail) = after
            .split_once("}}")
            .with_context(|| format!("unclosed {{{{ in {recipe}: {line}"))?;
        stages.extend(expand_stage_list(expression)?);
        rest = tail;
    }
    stages.extend(rest.split_whitespace().map(str::to_string));
    Ok(stages)
}

/// The stages a `{{…}}` expression of a recipe names: the justfile variable it reads, with the
/// stage a `replace(…)` swaps for the loop's `"$stage"` swapped the same way.
pub(super) fn expand_stage_list(expression: &str) -> Result<Vec<String>> {
    let variable = expression
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .find(|word| word.contains('_'))
        .with_context(|| format!("no variable in {expression}"))?;
    let output = just(&["--evaluate", variable], None)?;
    ensure!(output.status.success(), "{}", text(&output));
    let replaced = replaced_stage(expression);
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(|stage| {
            if replaced.as_deref() == Some(stage) {
                "\"$stage\"".to_string()
            } else {
                stage.to_string()
            }
        })
        .collect())
}

/// The stage a `replace(…)` of `expression` swaps for the loop's `"$stage"`.
fn replaced_stage(expression: &str) -> Option<String> {
    let (_, args) = expression.split_once("replace(")?;
    let quoted: Vec<&str> = args.split('"').collect();
    (quoted.len() >= 3).then(|| quoted[1].to_string())
}

/// A stage `_stages` runs under the machine-wide lock, by its rule: an E2E stage and every stage
/// after the first of them.
pub(super) fn waits_for_the_lock(stage: &str) -> bool {
    stage.starts_with("e2e") || stage == "\"$stage\""
}

pub(super) fn run_output(world: &E2eWorld) -> Result<&Output> {
    world.distribution.run.as_ref().context("nothing was run")
}
