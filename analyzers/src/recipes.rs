//! The recipe check: shellcheck over the shell recipes of the `justfile` and perlcritic over its Perl recipes,
//! their findings counted per `<tool> <recipe> <code>`.

use crate::baseline::Counts;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

/// What an `{{…}}` interpolation becomes in the script: one plain word.
const PLACEHOLDER: &str = "JUST_VALUE";

/// The language a recipe body runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Bash,
    /// A recipe without shebang: each line runs in `sh`.
    Sh,
    Perl,
}

/// One recipe body as the script its interpreter runs.
#[derive(Debug, PartialEq, Eq)]
pub struct Script {
    pub recipe: String,
    pub language: Language,
    pub text: String,
}

/// Dumps the `justfile` and checks every recipe body.
pub fn run() -> Result<Counts, String> {
    let dump = output(Command::new("just").args(["--dump", "--dump-format", "json"]))?;
    let dump: Value = serde_json::from_str(&dump).map_err(|e| format!("just --dump: {e}"))?;
    let dir = std::env::temp_dir().join(format!("voicepuncher-recipes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let counts = count_findings(&scripts(&dump), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    counts
}

/// The findings of every one of `scripts`, each written to a file of its own in `dir`.
fn count_findings(scripts: &[Script], dir: &Path) -> Result<Counts, String> {
    let mut counts = Counts::new();
    for script in scripts {
        let file = dir.join(script.recipe.replace('/', "_"));
        std::fs::write(&file, &script.text).map_err(|e| format!("{}: {e}", file.display()))?;
        for code in check(script, &file)? {
            *counts.entry(code).or_default() += 1;
        }
    }
    Ok(counts)
}

fn check(script: &Script, file: &Path) -> Result<Vec<String>, String> {
    let recipe = &script.recipe;
    Ok(match script.language {
        Language::Perl => {
            let out = output(
                Command::new("perlcritic")
                    .args(["--quiet", "--verbose", "%p\\n"])
                    .arg(file),
            )?;
            keys("perlcritic", recipe, perlcritic_policies(&out))
        }
        Language::Bash | Language::Sh => {
            let dialect = if script.language == Language::Bash {
                "bash"
            } else {
                "sh"
            };
            let out = output(
                Command::new("shellcheck")
                    .args(["-f", "json1", "-s", dialect])
                    .arg(file),
            )?;
            let codes = shellcheck_codes(&out)?
                .into_iter()
                .map(|c| format!("SC{c}"));
            keys("shellcheck", recipe, codes)
        }
    })
}

/// The finding keys of one recipe: `<tool> <recipe> <code>`.
pub fn keys(tool: &str, recipe: &str, codes: impl IntoIterator<Item = String>) -> Vec<String> {
    codes
        .into_iter()
        .map(|code| format!("{tool} {recipe} {code}"))
        .collect()
}

/// Runs a command and returns its standard output; a finding is no failure, a missing program is.
fn output(command: &mut Command) -> Result<String, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let out = command.output().map_err(|e| format!("{program}: {e}"))?;
    if out.stdout.is_empty() && !out.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Every recipe of a `just --dump --dump-format json` as a script, interpolations replaced by a placeholder.
pub fn scripts(dump: &Value) -> Vec<Script> {
    let Some(recipes) = dump.get("recipes").and_then(Value::as_object) else {
        return Vec::new();
    };
    recipes
        .iter()
        .filter_map(|(name, recipe)| script(name, recipe))
        .collect()
}

/// The script of the recipe `name`; none for a body in a language no tool checks.
fn script(name: &str, recipe: &Value) -> Option<Script> {
    let lines: Vec<String> = recipe.get("body")?.as_array()?.iter().map(line).collect();
    let shebang = recipe
        .get("shebang")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !shebang {
        let body: Vec<&str> = lines
            .iter()
            .map(|l| l.trim_start_matches(['@', '-']))
            .collect();
        let text = format!("#!/bin/sh\n{}\n", body.join("\n"));
        return Some(Script { recipe: name.to_owned(), language: Language::Sh, text });
    }
    let language = shebang_language(lines.first()?)?;
    let text = lines.join("\n") + "\n";
    Some(Script { recipe: name.to_owned(), language, text })
}

/// The language a shebang line names; none for one no tool checks.
fn shebang_language(first: &str) -> Option<Language> {
    if first.contains("perl") {
        Some(Language::Perl)
    } else if first.contains("sh") {
        Some(Language::Bash)
    } else {
        None
    }
}

/// One body line: its text fragments, each interpolation as the placeholder.
fn line(fragments: &Value) -> String {
    fragments
        .as_array()
        .into_iter()
        .flatten()
        .map(|fragment| fragment.as_str().unwrap_or(PLACEHOLDER))
        .collect()
}

/// The codes of shellcheck's `-f json1` report.
pub fn shellcheck_codes(report: &str) -> Result<Vec<u64>, String> {
    let report: Value =
        serde_json::from_str(report).map_err(|e| format!("shellcheck report: {e}"))?;
    Ok(report
        .get("comments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("code").and_then(Value::as_u64))
        .collect())
}

/// The policies of perlcritic's `--verbose "%p\n"` report, one per line.
pub fn perlcritic_policies(report: &str) -> Vec<String> {
    report
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.ends_with("source OK"))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump() -> Value {
        serde_json::json!({"recipes": {
            "lint": {"shebang": false, "body": [["@", [["variable", "locked"]], " _lint"]]},
            "_check": {"shebang": true, "body": [["#!/usr/bin/env bash"], ["echo ", [["variable", "x"]]]]},
            "_keys": {"shebang": true, "body": [["#!/usr/bin/env perl"], ["print 1;"]]},
            "_py": {"shebang": true, "body": [["#!/usr/bin/env python3"], ["print(1)"]]}
        }})
    }

    #[test]
    fn should_turn_recipes_into_scripts_with_interpolations_replaced() {
        let scripts = scripts(&dump());
        let by_name = |n: &str| scripts.iter().find(|s| s.recipe == n).expect("recipe");
        assert_eq!(by_name("lint").language, Language::Sh);
        assert_eq!(by_name("lint").text, "#!/bin/sh\nJUST_VALUE _lint\n");
        assert_eq!(by_name("_check").language, Language::Bash);
        assert_eq!(
            by_name("_check").text,
            "#!/usr/bin/env bash\necho JUST_VALUE\n"
        );
        assert_eq!(by_name("_keys").language, Language::Perl);
        assert!(scripts.iter().all(|s| s.recipe != "_py"));
    }

    #[test]
    fn should_read_the_codes_of_a_shellcheck_report() {
        let report =
            r#"{"comments":[{"code":2086,"level":"info"},{"code":2154,"level":"warning"}]}"#;
        assert_eq!(shellcheck_codes(report), Ok(vec![2086, 2154]));
    }

    #[test]
    fn should_name_the_tool_the_recipe_and_the_code_of_a_finding() {
        assert_eq!(
            keys("shellcheck", "_lint", ["SC2086".to_owned()]),
            vec!["shellcheck _lint SC2086"]
        );
    }

    #[test]
    fn should_read_the_policies_of_a_perlcritic_report() {
        let report = "Subroutines::ProhibitExplicitReturnUndef\nInputOutput::ProhibitTwoArgOpen\n";
        assert_eq!(
            perlcritic_policies(report),
            vec![
                "Subroutines::ProhibitExplicitReturnUndef",
                "InputOutput::ProhibitTwoArgOpen"
            ]
        );
    }
}
