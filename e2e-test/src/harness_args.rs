//! How a Gherkin harness (`harness = false`) reads its command line.
//!
//! cargo-nextest finds and runs tests through libtest's arguments; the harness answers them as one
//! test named [`TEST_NAME`], the whole scenario run, so nextest's time limit applies to it; the
//! slice and component harness answers them with one test per rule ([`read_rule_harness_call`]).
//! Every other command line is cucumber's own, as `cargo test --test <harness> -- --name …` passes
//! it.

/// The one test a harness shows nextest: its whole scenario run.
pub const TEST_NAME: &str = "scenarios";

/// What the harness was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessCall {
    /// nextest lists the tests: print each line on stdout, nothing else, and exit 0.
    Listing(Vec<String>),
    /// Run the scenarios with these cucumber arguments (without the program name).
    Run(Vec<String>),
}

/// Reads the harness's arguments, without the program name.
pub fn read_harness_call(args: &[String]) -> HarnessCall {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse"] => HarnessCall::Listing(vec![format!("{TEST_NAME}: test")]),
        ["--list", "--format", "terse", "--ignored"] => HarnessCall::Listing(vec![]),
        ["--exact", name, "--nocapture", ..] if *name == TEST_NAME => {
            HarnessCall::Run(args[3..].to_vec())
        }
        _ => HarnessCall::Run(args.to_vec()),
    }
}

/// The cucumber command line of this process, program name first, or `None` when nextest only
/// lists the tests: the listing is then printed on stdout and the harness exits with 0.
pub fn cucumber_command_line() -> Option<Vec<String>> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match read_harness_call(&rest) {
        HarnessCall::Listing(lines) => {
            for line in lines {
                println!("{line}");
            }
            None
        }
        HarnessCall::Run(cucumber) => Some(std::iter::once(program).chain(cucumber).collect()),
    }
}

/// The variable `check` and `full-check` set to `1`, so the harness stops at the first failed
/// scenario (openspec change fail-fast-stage-order).
pub const FAIL_FAST_VAR: &str = "VOICEPUNCHER_FAIL_FAST";

/// The cucumber command line with cucumber's `--fail-fast` added when the value of
/// [`FAIL_FAST_VAR`] is `1`; unchanged otherwise.
pub fn with_fail_fast(
    mut command_line: Vec<String>,
    var: &Result<String, std::env::VarError>,
) -> Vec<String> {
    if var.as_deref() == Ok("1") {
        command_line.push("--fail-fast".to_string());
    }
    command_line
}

/// The variable that sets how many scenarios the E2E harness runs at once.
pub const CONCURRENCY_VAR: &str = "VOICEPUNCHER_E2E_CONCURRENCY";
/// Scenarios the E2E harness runs at once without the variable: the fastest green value of
/// three smoke runs each at 4, 6 and 8 (openspec change speed-up-tests, baseline.md).
const DEFAULT_CONCURRENT_SCENARIOS: usize = 6;

/// How many scenarios run at once, from the value of [`CONCURRENCY_VAR`]: a positive whole
/// number, or the default when it is not set.
pub fn concurrent_scenarios(var: Result<String, std::env::VarError>) -> Result<usize, String> {
    match var {
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_CONCURRENT_SCENARIOS),
        Err(e) => Err(format!("{CONCURRENCY_VAR}: {e}")),
        Ok(value) => match value.trim().parse::<usize>() {
            Ok(n) if n > 0 => Ok(n),
            _ => Err(format!(
                "{CONCURRENCY_VAR} is {value:?}, not a positive whole number"
            )),
        },
    }
}

/// One test the slice and component harness shows nextest: the scenarios of one rule of a
/// feature file, or the scenarios outside any rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleTest {
    /// The test's name: the feature file's path below the features folder without `.feature`,
    /// then `/` and the rule's [`rule_slug`] for a rule.
    pub name: String,
    /// The feature file's path below the features folder without `.feature`.
    pub feature: String,
    /// The rule's name as the file writes it; `None` for the scenarios outside any rule.
    pub rule: Option<String>,
}

impl RuleTest {
    /// Whether a scenario of the test's feature file in the rule named `rule` (`None` outside
    /// any rule) belongs to this test.
    pub fn takes(&self, rule: Option<&str>) -> bool {
        self.rule.as_deref().map(str::trim) == rule.map(str::trim)
    }
}

/// A rule's name as a test name: lowercase letters and digits, every other run of characters
/// one `_`, so nextest filters and `just rerun-failed` take it unquoted.
pub fn rule_slug(rule: &str) -> String {
    rule.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

/// The tests of the feature file `feature` whose text is `text`: one per rule, plus one named
/// by the file for the scenarios outside any rule when it has any (every scenario of a file
/// without rules).
pub fn rule_tests(feature: &str, text: &str) -> Vec<RuleTest> {
    let mut rules: Vec<String> = Vec::new();
    let mut outside = false;
    for line in text.lines().map(str::trim_start) {
        if let Some(rule) = line.strip_prefix("Rule:") {
            rules.push(rule.trim().to_string());
        } else if rules.is_empty()
            && SCENARIO_KEYWORDS
                .iter()
                .any(|keyword| line.starts_with(keyword))
        {
            outside = true;
        }
    }
    let file_test = (outside || rules.is_empty()).then(|| RuleTest {
        name: feature.to_string(),
        feature: feature.to_string(),
        rule: None,
    });
    file_test
        .into_iter()
        .chain(rules.into_iter().map(|rule| RuleTest {
            name: format!("{feature}/{}", rule_slug(&rule)),
            feature: feature.to_string(),
            rule: Some(rule),
        }))
        .collect()
}

/// The keywords that open a scenario in a feature file.
const SCENARIO_KEYWORDS: [&str; 4] = [
    "Scenario:",
    "Scenario Outline:",
    "Scenario Template:",
    "Example:",
];

/// What a harness that shows one test per rule was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCall {
    /// nextest lists the tests: print each line on stdout, nothing else, and exit 0.
    Listing(Vec<String>),
    /// Run the scenarios of one test (every scenario when `None`) with these cucumber
    /// arguments (without the program name).
    Run { test: Option<RuleTest>, cucumber: Vec<String> },
}

/// Reads the arguments of a harness that shows nextest one test per rule ([`rule_tests`]), so
/// nextest runs the rules in parallel processes; every other command line runs every feature
/// file.
pub fn read_rule_harness_call(args: &[String], tests: &[RuleTest]) -> RuleCall {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse"] => {
            RuleCall::Listing(tests.iter().map(|t| format!("{}: test", t.name)).collect())
        }
        ["--list", "--format", "terse", "--ignored"] => RuleCall::Listing(vec![]),
        ["--exact", name, "--nocapture", ..] if tests.iter().any(|t| t.name == *name) => {
            RuleCall::Run {
                test: tests.iter().find(|t| t.name == *name).cloned(),
                cucumber: args[3..].to_vec(),
            }
        }
        _ => RuleCall::Run { test: None, cucumber: args.to_vec() },
    }
}

/// The feature files below `root`, each named by its path below `root` without `.feature`,
/// sorted.
pub fn feature_names(root: &std::path::Path) -> Vec<String> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, names: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                walk(&path, root, names);
                continue;
            }
            let feature = path.with_extension("");
            let relative = feature.strip_prefix(root).ok();
            if let Some(relative) =
                relative.filter(|_| path.extension().is_some_and(|e| e == "feature"))
            {
                names.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    let mut names = Vec::new();
    walk(root, root, &mut names);
    names.sort();
    names
}

/// The tests of every feature file below `root` ([`rule_tests`]), in the order of
/// [`feature_names`].
pub fn harness_tests(root: &std::path::Path) -> Vec<RuleTest> {
    feature_names(root)
        .into_iter()
        .flat_map(|feature| {
            let text = std::fs::read_to_string(root.join(format!("{feature}.feature")))
                .unwrap_or_default();
            rule_tests(&feature, &text)
        })
        .collect()
}

/// The test to run (every scenario when `None`) and the cucumber command line of this process,
/// program name first, or `None` when nextest only lists the tests.
///
/// The listing is then printed on stdout and the harness exits with 0.
pub fn rule_command_line(tests: &[RuleTest]) -> Option<(Option<RuleTest>, Vec<String>)> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match read_rule_harness_call(&rest, tests) {
        RuleCall::Listing(lines) => {
            for line in lines {
                println!("{line}");
            }
            None
        }
        RuleCall::Run { test, cucumber } => {
            Some((test, std::iter::once(program).chain(cucumber).collect()))
        }
    }
}

#[cfg(test)]
mod tests;
