//! How a Gherkin harness (`harness = false`) reads its command line. cargo-nextest finds and
//! runs tests through libtest's arguments; the harness answers them as one test named
//! [`TEST_NAME`], the whole scenario run, so nextest's time limit applies to it; the slice and
//! component harness answers them with one test per feature file ([`read_feature_harness_call`]).
//! Every other command line is cucumber's own, as `cargo test --test <harness> -- --name …`
//! passes it.

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
    var: Result<String, std::env::VarError>,
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

/// What a harness that shows one test per feature file was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureCall {
    /// nextest lists the tests: print each line on stdout, nothing else, and exit 0.
    Listing(Vec<String>),
    /// Run the scenarios of one feature file (all of them when `None`) with these cucumber
    /// arguments (without the program name).
    Run {
        feature: Option<String>,
        cucumber: Vec<String>,
    },
}

/// Reads the arguments of a harness that shows nextest one test per feature file, named by the
/// file's path below the features folder without `.feature`, so nextest runs the files in
/// parallel processes; every other command line runs every feature file.
pub fn read_feature_harness_call(args: &[String], features: &[String]) -> FeatureCall {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse"] => {
            FeatureCall::Listing(features.iter().map(|f| format!("{f}: test")).collect())
        }
        ["--list", "--format", "terse", "--ignored"] => FeatureCall::Listing(vec![]),
        ["--exact", name, "--nocapture", ..] if features.iter().any(|f| f == name) => {
            FeatureCall::Run {
                feature: Some(name.to_string()),
                cucumber: args[3..].to_vec(),
            }
        }
        _ => FeatureCall::Run {
            feature: None,
            cucumber: args.to_vec(),
        },
    }
}

/// The feature files below `root`, each named by its path below `root` without `.feature`,
/// sorted.
pub fn feature_names(root: &std::path::Path) -> Vec<String> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, names: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, names);
            } else if path.extension().is_some_and(|e| e == "feature") {
                if let Ok(relative) = path.with_extension("").strip_prefix(root) {
                    names.push(relative.to_string_lossy().into_owned());
                }
            }
        }
    }
    let mut names = Vec::new();
    walk(root, root, &mut names);
    names.sort();
    names
}

/// The feature file to run (all when `None`) and the cucumber command line of this process,
/// program name first, or `None` when nextest only lists the tests: the listing is then printed
/// on stdout and the harness exits with 0.
pub fn feature_command_line(features: &[String]) -> Option<(Option<String>, Vec<String>)> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match read_feature_harness_call(&rest, features) {
        FeatureCall::Listing(lines) => {
            for line in lines {
                println!("{line}");
            }
            None
        }
        FeatureCall::Run { feature, cucumber } => {
            Some((feature, std::iter::once(program).chain(cucumber).collect()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn should_add_fail_fast_to_the_cucumber_command_line_when_the_variable_is_one() {
        assert_eq!(
            with_fail_fast(args(&["prog", "--tags", "@smoke"]), Ok("1".to_string())),
            args(&["prog", "--tags", "@smoke", "--fail-fast"])
        );
    }

    #[test]
    fn should_keep_the_cucumber_command_line_when_the_variable_is_not_one() {
        let line = args(&["prog", "--tags", "@smoke"]);
        assert_eq!(
            with_fail_fast(line.clone(), Err(std::env::VarError::NotPresent)),
            line
        );
        assert_eq!(with_fail_fast(line.clone(), Ok("0".to_string())), line);
        assert_eq!(with_fail_fast(line.clone(), Ok(String::new())), line);
    }

    #[test]
    fn should_list_one_test_named_scenarios_when_nextest_lists_the_tests() {
        assert_eq!(
            read_harness_call(&args(&["--list", "--format", "terse"])),
            HarnessCall::Listing(vec!["scenarios: test".to_string()])
        );
    }

    #[test]
    fn should_list_no_test_when_nextest_lists_the_ignored_tests() {
        assert_eq!(
            read_harness_call(&args(&["--list", "--format", "terse", "--ignored"])),
            HarnessCall::Listing(vec![])
        );
    }

    #[test]
    fn should_keep_only_the_extra_arguments_when_nextest_runs_the_scenarios() {
        assert_eq!(
            read_harness_call(&args(&[
                "--exact",
                "scenarios",
                "--nocapture",
                "--tags",
                "@smoke and not @wip",
            ])),
            HarnessCall::Run(args(&["--tags", "@smoke and not @wip"]))
        );
    }

    #[test]
    fn should_keep_no_argument_when_nextest_runs_the_scenarios_without_extra_arguments() {
        assert_eq!(
            read_harness_call(&args(&["--exact", "scenarios", "--nocapture"])),
            HarnessCall::Run(vec![])
        );
    }

    #[test]
    fn should_pass_any_other_command_line_to_cucumber_unchanged() {
        for other in [
            args(&["--name", "the cat is fed"]),
            args(&["--tags", "@x"]),
            args(&[]),
            args(&["--exact", "another", "--nocapture"]),
            args(&["--list"]),
        ] {
            assert_eq!(read_harness_call(&other), HarnessCall::Run(other.clone()));
        }
    }

    fn features() -> Vec<String> {
        args(&["component/voice_control", "slice/text_insertion"])
    }

    #[test]
    fn should_list_one_test_per_feature_file_when_nextest_lists_the_tests() {
        assert_eq!(
            read_feature_harness_call(&args(&["--list", "--format", "terse"]), &features()),
            FeatureCall::Listing(args(&[
                "component/voice_control: test",
                "slice/text_insertion: test"
            ]))
        );
    }

    #[test]
    fn should_list_no_feature_when_nextest_lists_the_ignored_tests() {
        assert_eq!(
            read_feature_harness_call(
                &args(&["--list", "--format", "terse", "--ignored"]),
                &features()
            ),
            FeatureCall::Listing(vec![])
        );
    }

    #[test]
    fn should_run_only_the_feature_file_a_test_names() {
        assert_eq!(
            read_feature_harness_call(
                &args(&[
                    "--exact",
                    "slice/text_insertion",
                    "--nocapture",
                    "--tags",
                    "@x"
                ]),
                &features()
            ),
            FeatureCall::Run {
                feature: Some("slice/text_insertion".to_string()),
                cucumber: args(&["--tags", "@x"]),
            }
        );
    }

    #[test]
    fn should_run_every_feature_file_for_any_other_command_line() {
        for other in [
            args(&["--name", "the cat is fed"]),
            args(&[]),
            args(&["--exact", "component/unknown", "--nocapture"]),
        ] {
            assert_eq!(
                read_feature_harness_call(&other, &features()),
                FeatureCall::Run {
                    feature: None,
                    cucumber: other.clone(),
                }
            );
        }
    }

    #[test]
    fn should_name_each_feature_file_by_its_path_below_the_features_folder() {
        let root = std::env::temp_dir().join(format!("harness-args-{}", std::process::id()));
        std::fs::create_dir_all(root.join("slice")).unwrap();
        std::fs::create_dir_all(root.join("component")).unwrap();
        std::fs::write(root.join("slice/text_insertion.feature"), "").unwrap();
        std::fs::write(root.join("component/voice_control.feature"), "").unwrap();
        std::fs::write(root.join("component/notes.md"), "").unwrap();
        let names = feature_names(&root);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(names, features());
    }

    #[test]
    fn should_run_six_scenarios_at_once_without_the_concurrency_variable() {
        assert_eq!(
            concurrent_scenarios(Err(std::env::VarError::NotPresent)),
            Ok(6)
        );
    }

    #[test]
    fn should_run_as_many_scenarios_at_once_as_the_concurrency_variable_says() {
        assert_eq!(concurrent_scenarios(Ok("3".to_string())), Ok(3));
    }

    #[test]
    fn should_refuse_a_concurrency_that_is_no_positive_number() {
        for wrong in ["0", "-1", "four", ""] {
            assert!(
                concurrent_scenarios(Ok(wrong.to_string())).is_err(),
                "{wrong:?} was taken"
            );
        }
    }
}
