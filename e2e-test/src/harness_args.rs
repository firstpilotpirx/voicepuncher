//! How a Gherkin harness (`harness = false`) reads its command line. cargo-nextest finds and
//! runs tests through libtest's arguments; the harness answers them as one test named
//! [`TEST_NAME`], the whole scenario run, so nextest's time limit applies to it. Every other
//! command line is cucumber's own, as `cargo test --test <harness> -- --name …` passes it.

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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
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
}
