use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| (*a).to_string()).collect()
}

#[test]
fn should_add_fail_fast_to_the_cucumber_command_line_when_the_variable_is_one() {
    assert_eq!(
        with_fail_fast(args(&["prog", "--tags", "@smoke"]), &Ok("1".to_string())),
        args(&["prog", "--tags", "@smoke", "--fail-fast"])
    );
}

#[test]
fn should_keep_the_cucumber_command_line_when_the_variable_is_not_one() {
    let line = args(&["prog", "--tags", "@smoke"]);
    assert_eq!(
        with_fail_fast(line.clone(), &Err(std::env::VarError::NotPresent)),
        line
    );
    assert_eq!(with_fail_fast(line.clone(), &Ok("0".to_string())), line);
    assert_eq!(with_fail_fast(line.clone(), &Ok(String::new())), line);
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
