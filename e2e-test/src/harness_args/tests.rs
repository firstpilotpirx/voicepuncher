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

fn rule_test(name: &str, feature: &str, rule: Option<&str>) -> RuleTest {
    RuleTest {
        name: name.to_string(),
        feature: feature.to_string(),
        rule: rule.map(str::to_string),
    }
}

fn tests() -> Vec<RuleTest> {
    vec![
        rule_test(
            "component/voice_control/press_enter_presses_return",
            "component/voice_control",
            Some("\"press enter\" presses Return"),
        ),
        rule_test("slice/text_insertion", "slice/text_insertion", None),
    ]
}

#[test]
fn should_list_one_test_per_rule_when_nextest_lists_the_tests() {
    assert_eq!(
        read_rule_harness_call(&args(&["--list", "--format", "terse"]), &tests()),
        RuleCall::Listing(args(&[
            "component/voice_control/press_enter_presses_return: test",
            "slice/text_insertion: test"
        ]))
    );
}

#[test]
fn should_list_no_rule_when_nextest_lists_the_ignored_tests() {
    assert_eq!(
        read_rule_harness_call(
            &args(&["--list", "--format", "terse", "--ignored"]),
            &tests()
        ),
        RuleCall::Listing(vec![])
    );
}

#[test]
fn should_run_only_the_rule_a_test_names() {
    assert_eq!(
        read_rule_harness_call(
            &args(&[
                "--exact",
                "component/voice_control/press_enter_presses_return",
                "--nocapture",
                "--tags",
                "@x"
            ]),
            &tests()
        ),
        RuleCall::Run { test: Some(tests()[0].clone()), cucumber: args(&["--tags", "@x"]) }
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
            read_rule_harness_call(&other, &tests()),
            RuleCall::Run { test: None, cucumber: other.clone() }
        );
    }
}

#[test]
fn should_name_a_rule_by_its_lowercase_words_joined_by_underscores() {
    assert_eq!(
        rule_slug("  \"press enter\" during a dictation pastes and sends "),
        "press_enter_during_a_dictation_pastes_and_sends"
    );
    assert_eq!(
        rule_slug("An utterance ends after 0.3 s of silence"),
        "an_utterance_ends_after_0_3_s_of_silence"
    );
}

#[test]
fn should_show_one_test_per_rule_of_a_feature_file() {
    let text = "Feature: voice_control\n\n  Background:\n    Given x\n\n  Rule: \"press enter\" presses Return\n\n    Scenario: a\n      Given x\n\n  Rule: Each state accepts its own commands\n\n    Scenario Outline: b\n      Given x\n\n      Examples:\n        | a |\n";
    assert_eq!(
        rule_tests("component/voice_control", text),
        vec![
            tests()[0].clone(),
            rule_test(
                "component/voice_control/each_state_accepts_its_own_commands",
                "component/voice_control",
                Some("Each state accepts its own commands"),
            ),
        ]
    );
}

#[test]
fn should_keep_the_scenarios_outside_any_rule_under_the_file_name() {
    let text = "Feature: f\n\n  Scenario: outside\n    Given x\n\n  Rule: A rule\n\n    Example: inside\n      Given x\n";
    assert_eq!(
        rule_tests("slice/f", text),
        vec![
            rule_test("slice/f", "slice/f", None),
            rule_test("slice/f/a_rule", "slice/f", Some("A rule")),
        ]
    );
    assert_eq!(
        rule_tests("slice/g", "Feature: g\n\n  Scenario: only\n    Given x\n"),
        vec![rule_test("slice/g", "slice/g", None)]
    );
}

#[test]
fn should_take_only_the_scenarios_of_its_own_rule() {
    let [rule, outside] = [tests()[0].clone(), tests()[1].clone()];
    assert!(rule.takes(Some("\"press enter\" presses Return")));
    assert!(!rule.takes(Some("Each state accepts its own commands")));
    assert!(!rule.takes(None));
    assert!(outside.takes(None));
    assert!(!outside.takes(Some("A rule")));
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
    assert_eq!(
        names,
        args(&["component/voice_control", "slice/text_insertion"])
    );
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
